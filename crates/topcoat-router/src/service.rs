use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use hyper::body::Incoming;
use hyper::service::Service;

use crate::{Body, Request, Response, Router};

/// A [`hyper`] [`Service`] that dispatches each incoming request through a
/// [`Router`].
///
/// The wrapped [`Router`] is shared behind an [`Arc`], so the service is cheap
/// to clone. One clone is handed to each accepted connection.
#[derive(Clone)]
pub struct RouterService {
    router: Arc<Router>,
}

impl RouterService {
    /// Wraps `router` in a cloneable service.
    pub fn new(router: Router) -> Self {
        Self {
            router: Arc::new(router),
        }
    }
}

impl Service<Request<Incoming>> for RouterService {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        let router = self.router.clone();
        Box::pin(async move { Ok(router.handle(request.map(Body::new)).await) })
    }
}
