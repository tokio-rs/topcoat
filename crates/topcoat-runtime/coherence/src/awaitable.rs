use std::{
    pin::Pin,
    task::{Context, Poll},
};

use serde::{Serialize, Serializer, ser::Error};
use topcoat::runtime::{Surrogate, Surrogated};

/// A deterministic async fixture that can be captured by a runtime expression.
#[derive(Clone)]
pub struct Awaitable<T> {
    completion: Completion<T>,
    pending_once: bool,
}

impl<T> Awaitable<T> {
    /// A fixture that returns `value` when awaited.
    #[must_use]
    pub fn ready(value: T) -> Self {
        Self {
            completion: Completion::Return(value),
            pending_once: false,
        }
    }

    /// A fixture that panics when awaited.
    #[must_use]
    pub fn panicking(message: impl Into<String>) -> Self {
        Self {
            completion: Completion::Panic(message.into()),
            pending_once: false,
        }
    }

    /// Yields once before returning or panicking.
    #[must_use]
    pub fn after_yield(mut self) -> Self {
        self.pending_once = true;
        self
    }
}

impl<T> Surrogated for Awaitable<T> {
    type Surrogate = AwaitableSurrogate<T>;

    fn into_surrogate(self) -> Self::Surrogate {
        AwaitableSurrogate { state: Some(self) }
    }
}

/// The serializable future a captured [`Awaitable`] becomes inside `expr!`.
pub struct AwaitableSurrogate<T> {
    state: Option<Awaitable<T>>,
}

impl<T> Surrogate for AwaitableSurrogate<T> {
    type Real = Awaitable<T>;

    fn into_real(self) -> Self::Real {
        self.state.expect("awaitable was already consumed")
    }
}

impl<T: Surrogated + Unpin> Future for AwaitableSurrogate<T> {
    type Output = T::Surrogate;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let state = this
            .state
            .as_mut()
            .expect("awaitable polled after completion");
        if state.pending_once {
            state.pending_once = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        match this
            .state
            .take()
            .expect("awaitable state is present")
            .completion
        {
            Completion::Return(value) => Poll::Ready(value.into_surrogate()),
            Completion::Panic(message) => panic!("{message}"),
        }
    }
}

impl<T> Serialize for AwaitableSurrogate<T>
where
    T: Clone + Surrogated,
    T::Surrogate: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| S::Error::custom("awaitable was already consumed"))?;
        let completion = match &state.completion {
            Completion::Return(value) => Completion::Return(value.clone().into_surrogate()),
            Completion::Panic(message) => Completion::Panic(message.clone()),
        };
        EncodedAwaitable {
            t: "CoherenceFuture",
            pending_once: state.pending_once,
            completion,
        }
        .serialize(serializer)
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", content = "value")]
enum Completion<T> {
    Return(T),
    Panic(String),
}

#[derive(Serialize)]
struct EncodedAwaitable<T> {
    t: &'static str,
    pending_once: bool,
    completion: Completion<T>,
}
