use topcoat_router::RouterBuilder;

use crate::MailConfig;

/// Installs mail support on a [`RouterBuilder`].
pub trait RouterBuilderMailExt {
    /// Registers the mail `config` on the app context, so every handler can
    /// deliver mail with [`send`](crate::send).
    #[must_use]
    fn mail(self, config: MailConfig) -> Self;
}

impl RouterBuilderMailExt for RouterBuilder {
    fn mail(self, config: MailConfig) -> Self {
        self.app_context(config)
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::{context::Cx, error::Result};
    use topcoat_router::{
        Body, Method, Methods, Path, Request, Response, Route, RouteFuture, Router,
    };

    use crate::{Mail, MailConfig, Mailbox, MemoryTransport, RouterBuilderMailExt, send};

    struct SendMail;

    impl Route for SendMail {
        fn methods(&self) -> Methods<'_> {
            Methods::Only(&[Method::GET])
        }

        fn path(&self) -> &Path {
            Path::new("/")
        }

        fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
            Box::pin(async move {
                let mail = Mail::builder()
                    .from(Mailbox::new("ada@example.com")?)
                    .to([Mailbox::new("bob@example.com")?])
                    .subject("Hello")
                    .text("Hi")
                    .build();
                send(cx, mail).await?;
                Ok(Response::new(Body::empty()))
            })
        }
    }

    #[tokio::test]
    async fn handlers_send_through_the_registered_config() -> Result<()> {
        let transport = MemoryTransport::new();
        let router = Router::builder()
            .route(SendMail)
            .mail(MailConfig::builder().transport(transport.clone()).build())
            .build();
        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request should build");

        let response = router.handle(request).await;

        assert!(response.status().is_success(), "{}", response.status());
        assert_eq!(transport.sent().len(), 1);
        assert_eq!(transport.sent()[0].subject(), "Hello");
        Ok(())
    }
}
