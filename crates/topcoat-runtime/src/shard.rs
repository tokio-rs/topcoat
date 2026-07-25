use std::{fmt, hash::Hash, pin::Pin};

use futures_core::Stream;
use futures_util::{Sink, SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};
use topcoat_router::{
    Body, IntoResponse, Method, Methods, Path, PathBuf, Route, RouteFuture, RouterBuilder,
};
use topcoat_router::{
    FromRequest,
    websocket::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code},
};
use topcoat_view::View;

use crate::{Surrogate, Surrogated};

pub(crate) const SHARD_ROUTE_PREFIX: &str = "/_topcoat/shards";

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShardId(&'static str);

impl ShardId {
    #[must_use]
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0
    }
}

pub type ShardRenderFn =
    for<'cx> fn(
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<View, Error>> + Send + 'cx>>;

pub type WebSocketShardFn =
    for<'cx> fn(cx: &'cx Cx, socket: WebSocket) -> Pin<Box<dyn Future<Output = ()> + Send + 'cx>>;

#[derive(Clone, Copy)]
enum ErasedShardKind {
    Http(ShardRenderFn),
    WebSocket(WebSocketShardFn),
}

impl fmt::Debug for ErasedShardKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(_) => f.write_str("Http"),
            Self::WebSocket(_) => f.write_str("WebSocket"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ErasedShard {
    id: ShardId,
    kind: ErasedShardKind,
}

impl ErasedShard {
    #[must_use]
    pub const fn new(id: ShardId, render: ShardRenderFn) -> Self {
        Self {
            id,
            kind: ErasedShardKind::Http(render),
        }
    }

    #[must_use]
    pub const fn new_websocket(id: ShardId, connect: WebSocketShardFn) -> Self {
        Self {
            id,
            kind: ErasedShardKind::WebSocket(connect),
        }
    }

    #[must_use]
    pub fn id(&self) -> ShardId {
        self.id
    }

    /// Renders an HTTP shard for an endpoint request, deserializing its
    /// arguments from `body`.
    ///
    /// # Errors
    ///
    /// Propagates any error returned by the shard's render function. Returns
    /// an error when called for a WebSocket shard.
    #[inline]
    pub async fn render(&self, cx: &Cx, body: Body) -> Result<View> {
        match self.kind {
            ErasedShardKind::Http(render) => render(cx, body).await,
            ErasedShardKind::WebSocket(_) => Err(std::io::Error::other(
                "a WebSocket shard cannot be rendered from an HTTP body",
            )
            .into()),
        }
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ErasedShard);

pub struct ShardRoute {
    path: PathBuf,
    shard: ErasedShard,
}

impl ShardRoute {
    /// Builds the route that serves a shard.
    pub fn new(shard: impl Into<ErasedShard>) -> Self {
        let shard = shard.into();
        Self {
            path: Path::new(&format!("{SHARD_ROUTE_PREFIX}/{}", shard.id().as_str())).to_owned(),
            shard,
        }
    }
}

impl Route for ShardRoute {
    fn methods(&self) -> Methods<'_> {
        match self.shard.kind {
            // Avoids URL length limits for large parameters.
            ErasedShardKind::Http(_) => Methods::Only(&[Method::POST]),
            ErasedShardKind::WebSocket(_) => Methods::Only(&[Method::GET]),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Box::pin(async move {
            match self.shard.kind {
                ErasedShardKind::Http(render) => {
                    let view = render(cx, body).await?;
                    view.into_response(cx)
                }
                ErasedShardKind::WebSocket(connect) => {
                    let upgrade = WebSocketUpgrade::from_request(cx, body).await?;
                    upgrade.on_upgrade_with_context(move |cx, socket| async move {
                        connect(&cx, socket).await;
                    })
                }
            }
        })
    }
}

/// Registers shards on a [`RouterBuilder`].
pub trait RouterBuilderShardExt {
    /// Mounts a shard route.
    #[must_use]
    fn shard(self, shard: impl Into<ErasedShard>) -> Self;

    /// Registers every shard linked into the binary.
    #[cfg(feature = "discover")]
    #[must_use]
    fn discover_shards(self) -> Self;
}

impl RouterBuilderShardExt for RouterBuilder {
    fn shard(self, shard: impl Into<ErasedShard>) -> Self {
        self.route(ShardRoute::new(shard))
    }

    #[cfg(feature = "discover")]
    fn discover_shards(mut self) -> Self {
        for shard in inventory::iter::<ErasedShard>().copied() {
            self = self.shard(shard);
        }
        self
    }
}

/// Seeds the temporary capacity-one channel used for a WebSocket shard's
/// server-side render.
#[doc(hidden)]
pub async fn __websocket_shard_seed<A>(argument: A) -> Result<mpsc::Receiver<A>> {
    let (sender, receiver) = mpsc::channel(1);
    sender.send(argument).await.map_err(|_| {
        Error::from(std::io::Error::other(
            "failed to seed WebSocket shard input",
        ))
    })?;
    Ok(receiver)
}

/// Takes the first item from a WebSocket shard's server-side render stream.
#[doc(hidden)]
pub async fn __websocket_shard_first<S>(stream: S) -> Result<View>
where
    S: Stream<Item = Result<View>>,
{
    let mut stream = std::pin::pin!(stream);
    stream.next().await.ok_or_else(|| {
        Error::from(std::io::Error::other(
            "WebSocket shard stream ended before its initial render",
        ))
    })?
}

#[derive(Debug, Clone, Copy)]
enum WebSocketTermination {
    Disconnected,
    Close { code: u16, reason: &'static str },
}

/// Runs a persistent WebSocket shard connection. The `create_stream` future is
/// polled concurrently with incoming arguments, so it may await its receiver
/// before returning the output stream.
#[doc(hidden)]
pub async fn __run_websocket_shard<A, F, Fut, S>(cx: &Cx, socket: WebSocket, create_stream: F)
where
    A: Surrogated + Send,
    <(A,) as Surrogated>::Surrogate: DeserializeOwned,
    F: FnOnce(mpsc::Receiver<A>) -> Fut + Send,
    Fut: Future<Output = S> + Send,
    S: Stream<Item = Result<View>> + Send,
{
    let (mut output, mut input) = socket.split();
    let (sender, receiver) = mpsc::channel(1);
    let mut incoming = std::pin::pin!(receive_websocket_inputs::<A, _>(&mut input, sender));
    let stream = {
        let create_stream = create_stream(receiver);
        let mut create_stream = std::pin::pin!(create_stream);
        tokio::select! {
            termination = &mut incoming => {
                finish_websocket(&mut output, termination).await;
                return;
            }
            stream = &mut create_stream => stream,
        }
    };

    let mut stream = std::pin::pin!(stream);
    let termination = tokio::select! {
        termination = &mut incoming => termination,
        termination = send_websocket_outputs(cx, &mut output, stream.as_mut()) => termination,
    };
    finish_websocket(&mut output, termination).await;
}

async fn receive_websocket_inputs<A, S>(
    input: &mut S,
    sender: mpsc::Sender<A>,
) -> WebSocketTermination
where
    A: Surrogated + Send,
    <(A,) as Surrogated>::Surrogate: DeserializeOwned,
    S: Stream<Item = Result<Message>> + Unpin,
{
    type Wire<A> = <(A,) as Surrogated>::Surrogate;

    while let Some(message) = input.next().await {
        let Ok(message) = message else {
            return WebSocketTermination::Disconnected;
        };
        let text = match message {
            Message::Text(text) => text,
            Message::Binary(_) => {
                return WebSocketTermination::Close {
                    code: close_code::UNSUPPORTED,
                    reason: "shard arguments must be text",
                };
            }
            Message::Close(_) => return WebSocketTermination::Disconnected,
            Message::Ping(_) | Message::Pong(_) => continue,
        };

        let Ok(argument) = serde_json::from_str::<Wire<A>>(text.as_str()) else {
            return WebSocketTermination::Close {
                code: close_code::INVALID,
                reason: "invalid shard arguments",
            };
        };
        let (argument,) = Surrogate::into_real(argument);
        if sender.send(argument).await.is_err() {
            return WebSocketTermination::Close {
                code: close_code::NORMAL,
                reason: "shard input stream closed",
            };
        }
    }
    WebSocketTermination::Disconnected
}

async fn send_websocket_outputs<S, O>(
    cx: &Cx,
    output: &mut O,
    mut stream: Pin<&mut S>,
) -> WebSocketTermination
where
    S: Stream<Item = Result<View>>,
    O: Sink<Message, Error = Error> + Unpin,
{
    while let Some(view) = stream.next().await {
        let Ok(view) = view else {
            return WebSocketTermination::Close {
                code: close_code::ERROR,
                reason: "shard stream failed",
            };
        };
        if output.send(Message::text(view.render(cx))).await.is_err() {
            return WebSocketTermination::Disconnected;
        }
    }
    WebSocketTermination::Close {
        code: close_code::NORMAL,
        reason: "shard stream completed",
    }
}

async fn finish_websocket<O>(output: &mut O, termination: WebSocketTermination)
where
    O: Sink<Message, Error = Error> + Unpin,
{
    match termination {
        WebSocketTermination::Disconnected => {
            let _ = output.close().await;
        }
        WebSocketTermination::Close { code, reason } => {
            let _ = output
                .send(Message::Close(Some(CloseFrame {
                    code,
                    reason: reason.into(),
                })))
                .await;
        }
    }
}
