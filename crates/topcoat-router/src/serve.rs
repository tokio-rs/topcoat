use std::future::Future;
use std::io;
use std::pin::pin;

use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::sync::watch;

use crate::{Listener, RouterService};

/// Serves a [`RouterService`] on an already-bound [`Listener`] until
/// `shutdown` completes.
///
/// This is the low-level accept loop, with no dev-server integration: it
/// accepts connections in a loop, serving each on its own task. When the
/// `shutdown` future completes, the listener is dropped and every open
/// connection finishes its in-flight request (up to the service's shutdown
/// timeout) before the call returns. Applications typically use the facade's
/// `serve`/`start` helpers, which layer a default shutdown signal and
/// dev-server readiness notification on top of this.
///
/// # Errors
///
/// Returns an I/O error if accepting a connection fails. Connections already
/// being served are left running on their tasks.
pub async fn internal_serve(
    mut listener: impl Listener,
    service: RouterService,
    shutdown: impl Future<Output = ()>,
) -> io::Result<()> {
    // Each channel signals by closing, not by sending: connection tasks hold
    // receiver clones, and dropping the sender resolves their `changed()`.
    //
    // Announces the shutdown; tasks switch into graceful shutdown.
    let (drain_tx, drain_rx) = watch::channel(());
    // Announces the end of the grace period; tasks drop their connection.
    let (cutoff_tx, cutoff_rx) = watch::channel(());
    // Tracks live tasks the other way around: each holds a receiver clone,
    // and `done_tx.closed()` resolves once the last one is dropped.
    let (done_tx, done_rx) = watch::channel(());

    let mut shutdown = pin!(shutdown);

    loop {
        let accepted = tokio::select! {
            accepted = listener.accept() => accepted,
            () = &mut shutdown => break,
        };
        let (stream, _remote) = accepted?;
        let io = TokioIo::new(stream);
        let service = service.clone();

        let mut drain_rx = drain_rx.clone();
        let mut cutoff_rx = cutoff_rx.clone();
        let done_rx = done_rx.clone();

        tokio::spawn(async move {
            // Held for the task's lifetime, so the shutdown sequence can wait
            // for connections to finish.
            let _done_rx = done_rx;

            let builder = auto::Builder::new(TokioExecutor::new());
            let mut connection = pin!(builder.serve_connection(io, service));

            let result = tokio::select! {
                result = connection.as_mut() => result,
                _ = drain_rx.changed() => {
                    // Finish the in-flight request, then close: HTTP/1 stops
                    // keep-alive, HTTP/2 sends GOAWAY.
                    connection.as_mut().graceful_shutdown();
                    tokio::select! {
                        result = connection.as_mut() => result,
                        // The grace period ended; drop the connection as is.
                        _ = cutoff_rx.changed() => return,
                    }
                }
            };

            if let Err(_error) = result {
                // TODO: Surface real connection errors without the noise. Most
                // are benign (e.g. the client aborting an in-flight shard
                // request), so for now they are dropped. See how axum demotes
                // these to a `trace`-level log.
            }
        });
    }

    // Stop listening while connections drain, freeing a TCP port for any
    // replacement process.
    drop(listener);

    // Tell every connection task to begin its graceful shutdown, and give
    // them the grace period to finish.
    drop(drain_rx);
    drop(drain_tx);
    drop(done_rx);
    tokio::select! {
        () = done_tx.closed() => {}
        () = tokio::time::sleep(service.shutdown_timeout) => {}
    }

    // Cut the connections that remain and wait for their tasks to end.
    drop(cutoff_rx);
    drop(cutoff_tx);
    done_tx.closed().await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::net::SocketAddr;
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::oneshot;
    use tokio::task::JoinHandle;
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{Body, IntoResponse, Method, Path, RouteFn, RouteFuture, RouteHandlerFn, Router};

    /// Builds a router with `handler` registered under `GET /x`.
    fn router_with(handler: RouteHandlerFn) -> Router {
        Router::builder()
            .route(RouteFn::new(
                Method::GET,
                Cow::Borrowed(Path::new("/x")),
                handler,
            ))
            .build()
    }

    fn say_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "served".into_response(cx) })
    }

    /// A route slow enough that a shutdown signal lands mid-request.
    fn slow_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            "slow".into_response(cx)
        })
    }

    /// A route that never resolves, holding its connection open forever.
    fn hang_route(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(std::future::pending())
    }

    /// Serves `service` on an ephemeral port, shutting down when the returned
    /// sender fires.
    async fn spawn_server(
        service: RouterService,
    ) -> (SocketAddr, oneshot::Sender<()>, JoinHandle<io::Result<()>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server = tokio::spawn(internal_serve(listener, service, async {
            let _ = shutdown_rx.await;
        }));
        (addr, shutdown_tx, server)
    }

    /// Waits for the server to return, bounded so a stuck shutdown fails the
    /// test instead of hanging it.
    async fn shut_down(server: JoinHandle<io::Result<()>>) {
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("server did not shut down within the grace period")
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn returns_once_the_shutdown_signal_fires() {
        let service = RouterService::new(router_with(say_route));
        let (addr, shutdown_tx, server) = spawn_server(service).await;

        // A roundtrip proves the server is up before it is shut down.
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /x HTTP/1.1\r\nhost: test\r\nconnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.ends_with("served"));

        shutdown_tx.send(()).unwrap();
        shut_down(server).await;

        // The listener is gone; new connections are refused.
        assert!(TcpStream::connect(addr).await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn serves_over_a_unix_socket() {
        use tokio::net::{UnixListener, UnixStream};

        let service = RouterService::new(router_with(say_route));
        let path = std::env::temp_dir().join(format!("topcoat-serve-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server = tokio::spawn(internal_serve(listener, service, async {
            let _ = shutdown_rx.await;
        }));

        let mut stream = UnixStream::connect(&path).await.unwrap();
        stream
            .write_all(b"GET /x HTTP/1.1\r\nhost: test\r\nconnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.ends_with("served"));

        shutdown_tx.send(()).unwrap();
        shut_down(server).await;

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn drains_the_in_flight_request_before_returning() {
        let service = RouterService::new(router_with(slow_route));
        let (addr, shutdown_tx, server) = spawn_server(service).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /x HTTP/1.1\r\nhost: test\r\n\r\n")
            .await
            .unwrap();

        // Let the request reach the route before the signal fires.
        tokio::time::sleep(Duration::from_millis(50)).await;
        shutdown_tx.send(()).unwrap();

        // The graceful shutdown lets the response finish, then closes the
        // kept-alive connection, ending the read.
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        assert!(response.contains("200 OK"));
        assert!(response.ends_with("slow"));

        shut_down(server).await;
    }

    #[tokio::test]
    async fn cuts_hung_connections_at_the_shutdown_timeout() {
        let service = RouterService::new(router_with(hang_route))
            .shutdown_timeout(Duration::from_millis(100));
        let (addr, shutdown_tx, server) = spawn_server(service).await;

        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET /x HTTP/1.1\r\nhost: test\r\n\r\n")
            .await
            .unwrap();

        // Let the request reach the route before the signal fires.
        tokio::time::sleep(Duration::from_millis(50)).await;
        shutdown_tx.send(()).unwrap();

        // The route never resolves, so the server returns at the timeout.
        shut_down(server).await;

        // The connection was cut without a response.
        let mut response = String::new();
        let result = stream.read_to_string(&mut response).await;
        assert!(result.is_err() || response.is_empty());
    }
}
