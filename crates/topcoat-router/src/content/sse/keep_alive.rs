use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use tokio::time::{Instant, Sleep};
use topcoat_core::error::Result;

use crate::content::sse::Event;

/// Configures the keep-alive events [`Sse`](crate::content::sse::Sse) sends while its
/// stream is idle.
///
/// Proxies and load balancers drop connections that look stale; a keep-alive
/// event whenever nothing was sent for [`interval`](Self::interval) keeps a
/// quiet stream open. The default sends an empty comment every 15 seconds.
#[derive(Clone, Debug)]
#[must_use]
pub struct KeepAlive {
    event: Event,
    interval: Duration,
}

impl KeepAlive {
    /// Creates the default configuration: an empty comment every 15 seconds.
    pub fn new() -> Self {
        /// Frequent enough for common proxy idle timeouts of a minute.
        const DEFAULT_INTERVAL: Duration = Duration::from_secs(15);

        Self {
            event: Event::new().comment(""),
            interval: DEFAULT_INTERVAL,
        }
    }

    /// Sets the idle time after which a keep-alive event is sent.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Sets the text of the keep-alive comment. Empty by default.
    pub fn text(self, text: impl Into<String>) -> Self {
        self.event(Event::new().comment(text))
    }

    /// Sends `event` as the keep-alive instead of a comment.
    pub fn event(mut self, event: Event) -> Self {
        self.event = event;
        self
    }

    /// Serializes the keep-alive event and prepares the idle timer.
    pub(super) fn into_timer(self) -> Result<KeepAliveTimer> {
        Ok(KeepAliveTimer {
            frame: self.event.serialize()?,
            interval: self.interval,
            sleep: None,
        })
    }
}

impl Default for KeepAlive {
    fn default() -> Self {
        Self::new()
    }
}

/// The keep-alive state of a running event stream body: the serialized
/// keep-alive event and the timer measuring idle time.
pub(super) struct KeepAliveTimer {
    frame: Bytes,
    interval: Duration,
    /// Armed on first poll, so building the response needs no timer runtime.
    sleep: Option<Pin<Box<Sleep>>>,
}

impl KeepAliveTimer {
    /// Polls the idle timer, yielding the keep-alive frame and restarting the
    /// timer once the stream has been idle for the interval.
    pub(super) fn poll_frame(&mut self, cx: &mut Context<'_>) -> Poll<Bytes> {
        let sleep = self
            .sleep
            .get_or_insert_with(|| Box::pin(tokio::time::sleep(self.interval)));
        match sleep.as_mut().poll(cx) {
            Poll::Ready(()) => {
                sleep.as_mut().reset(Instant::now() + self.interval);
                Poll::Ready(self.frame.clone())
            }
            Poll::Pending => Poll::Pending,
        }
    }

    /// Restarts the idle timer, called whenever the stream produced an event.
    pub(super) fn defer(&mut self) {
        if let Some(sleep) = &mut self.sleep {
            sleep.as_mut().reset(Instant::now() + self.interval);
        }
    }
}
