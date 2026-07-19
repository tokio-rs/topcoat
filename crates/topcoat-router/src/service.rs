use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use hyper::body::Incoming;
use hyper::service::Service;

use crate::{Body, Request, Response, Router};

/// A [`Router`] together with the configuration it is served with.
///
/// The serve functions accept any `impl Into<RouterService>`, so passing a
/// [`Router`] serves it with the defaults. Construct the service explicitly
/// to change how the router is served, like the graceful shutdown timeout:
///
/// ```
/// use std::time::Duration;
///
/// use topcoat::router::{Router, RouterService};
///
/// let service =
///     RouterService::new(Router::builder().build()).shutdown_timeout(Duration::from_secs(5));
/// ```
///
/// The wrapped [`Router`] is shared behind an [`Arc`], so the service is cheap
/// to clone. One clone is handed to each accepted connection.
#[derive(Clone)]
pub struct RouterService {
    router: Arc<Router>,
    pub(crate) shutdown_timeout: Duration,
}

impl RouterService {
    /// Wraps `router` in a cloneable service with the default configuration.
    pub fn new(router: Router) -> Self {
        /// How long in-flight requests get to finish by default.
        const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

        Self {
            router: Arc::new(router),
            shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
        }
    }

    /// Sets how long in-flight requests get to finish during a graceful
    /// shutdown before their connections are closed.
    ///
    /// After the shutdown signal, the server stops accepting connections and
    /// waits up to this long for open connections to complete their current
    /// request. The default is 30 seconds; [`Duration::ZERO`] closes all
    /// connections immediately.
    #[must_use]
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }
}

impl From<Router> for RouterService {
    fn from(router: Router) -> Self {
        Self::new(router)
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
