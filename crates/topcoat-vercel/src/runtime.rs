use std::{fmt, sync::Arc};

use http_body_util::{BodyExt, StreamBody};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use topcoat_router::{Body, Router};
use vercel_runtime::{Request, Response, ResponseBody, run as run_runtime, service_fn};

/// An error returned by the Vercel runtime.
#[derive(Debug)]
pub struct Error(vercel_runtime::Error);

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

/// Starts a Topcoat router using the Vercel Rust runtime.
///
/// The same entry point works under `vercel dev` and in a deployed Vercel
/// Function. Response frames are forwarded as they arrive so streaming views
/// are not buffered.
///
/// # Errors
///
/// Returns an error if the Vercel runtime cannot start or stops unexpectedly.
pub async fn run(router: Router) -> Result<(), Error> {
    let router = Arc::new(router);

    run_runtime(service_fn(move |request| {
        let router = Arc::clone(&router);
        async move { handle(router, request).await }
    }))
    .await
    .map_err(Error)
}

async fn handle(
    router: Arc<Router>,
    request: Request,
) -> Result<Response<ResponseBody>, vercel_runtime::Error> {
    let response = router.handle(request.map(Body::new)).await;
    let (parts, mut body) = response.into_parts();
    let (sender, receiver) = mpsc::channel(10);

    tokio::spawn(async move {
        while let Some(frame) = body.frame().await {
            if sender.send(frame).await.is_err() {
                break;
            }
        }
    });

    let body = StreamBody::new(ReceiverStream::new(receiver));
    Ok(Response::from_parts(parts, body.into()))
}
