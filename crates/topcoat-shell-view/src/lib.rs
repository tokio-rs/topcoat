#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
};

use bytes::Bytes;
use futures_util::{
    StreamExt as _,
    stream::{self, FuturesUnordered},
};
use http::{HeaderValue, header::CONTENT_TYPE};
use http_body::Frame;
use http_body_util::StreamBody;
use topcoat_core::{
    context::{Cx, CxHandle},
    error::Result,
};
use topcoat_router::{
    Body,
    response::{IntoResponse, Response},
};
use topcoat_view::{HtmlContext, PartsWriter, View, ViewParts};

type DeferredFuture = Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>>;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

const HTML: HeaderValue = HeaderValue::from_static("text/html; charset=utf-8");

/// Builds a streaming [`ShellView`] from a normal view shell and deferred views.
pub struct ShellViewBuilder {
    cx: CxHandle,
    deferred: Vec<DeferredFuture>,
}

impl ShellViewBuilder {
    /// Defers a view and returns its placeholder for insertion into the shell.
    ///
    /// `render` receives an owned request context handle. Its future starts
    /// when the response body is polled, and all deferred futures are polled
    /// concurrently. Completed views replace their placeholders immediately.
    pub fn defer<F, Fut>(&mut self, placeholder: View, render: F) -> View
    where
        F: FnOnce(CxHandle) -> Fut + Send + 'static,
        Fut: Future<Output = Result<View>> + Send + 'static,
    {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let cx = self.cx.clone();
        self.deferred.push(Box::pin(async move {
            let view = render(cx.clone()).await?;
            Ok(Bytes::from(patch(id, &view.render(&cx))))
        }));
        placeholder_view(id, placeholder)
    }

    /// Inserts a child shell view and adopts its deferred work.
    ///
    /// Put the returned shell view into the parent shell. Deferred fragments
    /// from both views then share the same response stream.
    #[must_use]
    pub fn include(&mut self, child: ShellView) -> View {
        self.deferred.extend(child.deferred);
        child.shell
    }

    /// Finishes the builder with the view sent as the first response chunk.
    #[must_use]
    pub fn finish(self, shell: View) -> ShellView {
        ShellView {
            shell,
            deferred: self.deferred,
        }
    }
}

/// An HTML response that streams deferred views into placeholders.
pub struct ShellView {
    shell: View,
    deferred: Vec<DeferredFuture>,
}

impl ShellView {
    /// Starts a shell view builder for `cx`.
    #[must_use]
    pub fn builder(cx: &Cx) -> ShellViewBuilder {
        ShellViewBuilder {
            cx: cx.handle(),
            deferred: Vec::new(),
        }
    }

    /// Creates a shell view with no deferred fragments.
    #[must_use]
    pub fn from_view(view: View) -> Self {
        Self {
            shell: view,
            deferred: Vec::new(),
        }
    }
}

impl IntoResponse for ShellView {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let rendered = self.shell.render_response(cx);
        let initial = stream::once(async move {
            Ok::<_, topcoat_core::error::Error>(Frame::data(Bytes::from(rendered.html)))
        });
        let deferred = self
            .deferred
            .into_iter()
            .collect::<FuturesUnordered<_>>()
            .map(|result| result.map(Frame::data));
        let body = Body::new(StreamBody::new(initial.chain(deferred)));
        let mut response = Response::new(body);
        *response.status_mut() = rendered.status_code.unwrap_or_default();
        response.headers_mut().insert(CONTENT_TYPE, HTML);
        response.headers_mut().extend(rendered.headers);
        Ok(response)
    }
}

fn placeholder_view(id: u64, placeholder: View) -> View {
    let mut parts = ViewParts::new();
    PartsWriter::new(&mut parts, HtmlContext::Unescaped).push_str_unescaped(format!(
        r#"<topcoat-shell id="topcoat-shell-{id}" style="display: contents">"#,
    ));
    parts.push_view(placeholder);
    PartsWriter::new(&mut parts, HtmlContext::Unescaped).push_str_unescaped("</topcoat-shell>");
    View::new(parts)
}

fn patch(id: u64, html: &str) -> String {
    let html = serde_json::to_string(html)
        .expect("serializing a string cannot fail")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    format!(
        r#"<script>(()=>{{const p=document.getElementById("topcoat-shell-{id}");if(!p)return;const t=document.createElement("template");t.innerHTML={html};p.replaceWith(t.content)}})()</script>"#,
    )
}

#[cfg(test)]
mod tests {
    use std::{
        future::{pending, poll_fn},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        task::Poll,
        time::Duration,
    };

    use futures_util::StreamExt as _;
    use topcoat::{
        context::{Cx, request_context},
        router::{response::IntoResponse as _, to_bytes},
        shell_view::ShellView,
        view::view,
    };

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn streams_shell_then_fragments_in_completion_order() {
        let cx = Cx::default();
        let cx_ref = &cx;
        let completed = Arc::new(Mutex::new(Vec::new()));
        let mut page = ShellView::builder(&cx);

        let slow_completed = Arc::clone(&completed);
        let slow = page.defer(
            view! { cx_ref => <p>"slow placeholder"</p> }.unwrap(),
            move |cx| async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                slow_completed.lock().unwrap().push("slow");
                let cx_ref = cx.as_ref();
                view! { cx_ref => <p>"slow result"</p> }
            },
        );
        let fast_completed = Arc::clone(&completed);
        let fast = page.defer(
            view! { cx_ref => <p>"fast placeholder"</p> }.unwrap(),
            move |cx| async move {
                fast_completed.lock().unwrap().push("fast");
                let cx_ref = cx.as_ref();
                view! { cx_ref => <p>"fast result"</p> }
            },
        );
        let response = page
            .finish(
                view! {
                    cx_ref =>
                    <main>
                        (slow)
                        (fast)
                    </main>
                }
                .unwrap(),
            )
            .into_response(&cx)
            .unwrap();
        let mut body = response.into_body().into_data_stream();

        let shell = String::from_utf8(body.next().await.unwrap().unwrap().to_vec()).unwrap();
        assert!(shell.contains("slow placeholder"));
        assert!(shell.contains("fast placeholder"));

        let first_patch = String::from_utf8(body.next().await.unwrap().unwrap().to_vec()).unwrap();
        assert!(first_patch.contains("fast result"));
        let second_patch = String::from_utf8(body.next().await.unwrap().unwrap().to_vec()).unwrap();
        assert!(second_patch.contains("slow result"));
        assert_eq!(*completed.lock().unwrap(), ["fast", "slow"]);
    }

    #[tokio::test]
    async fn deferred_view_keeps_request_context_alive() {
        struct Name(&'static str);

        let cx = topcoat::context::CxTestBuilder::new()
            .request_context(Name("Ada"))
            .build();
        let cx_ref = &cx;
        let mut page = ShellView::builder(&cx);
        let greeting = page.defer(view! { cx_ref => "Loading" }.unwrap(), |cx| async move {
            let name = request_context::<Name>(&cx).0;
            let cx_ref = cx.as_ref();
            view! {
                cx_ref =>
                <p>
                    "Hello, "
                    (name)
                </p>
            }
        });
        let response = page
            .finish(view! { cx_ref => (greeting) }.unwrap())
            .into_response(&cx)
            .unwrap();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Hello, "));
        assert!(html.contains("Ada"));
    }

    #[tokio::test]
    async fn dropping_response_cancels_deferred_views() {
        let cx = Cx::default();
        let cx_ref = &cx;
        let dropped = Arc::new(AtomicBool::new(false));
        let mut page = ShellView::builder(&cx);
        let future_dropped = Arc::clone(&dropped);
        let slot = page.defer(
            view! { cx_ref => "Loading" }.unwrap(),
            move |_cx| async move {
                let _drop_flag = DropFlag(future_dropped);
                pending::<()>().await;
                unreachable!()
            },
        );
        let response = page
            .finish(view! { cx_ref => (slot) }.unwrap())
            .into_response(&cx)
            .unwrap();
        let mut body = response.into_body().into_data_stream();

        body.next().await.unwrap().unwrap();
        poll_fn(|cx| {
            assert!(body.poll_next_unpin(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        assert!(!dropped.load(Ordering::SeqCst));

        drop(body);
        assert!(dropped.load(Ordering::SeqCst));
    }

    async fn shell_section(cx: &Cx, label: &'static str) -> topcoat::Result<ShellView> {
        let cx_ref = cx;
        let mut child = ShellView::builder(cx);
        let child_slot = child.defer(
            view! { cx_ref => <p>(format!("{label} loading"))</p> }?,
            move |cx| async move {
                let cx_ref = cx.as_ref();
                view! { cx_ref => <p>(format!("{label} ready"))</p> }
            },
        );
        Ok(child.finish(view! { cx_ref => <section>(child_slot)</section> }?))
    }

    #[tokio::test]
    async fn includes_multiple_shell_view_containers() {
        let cx = Cx::default();
        let cx_ref = &cx;
        let activity = shell_section(&cx, "activity").await.unwrap();
        let recommendations = shell_section(&cx, "recommendations").await.unwrap();

        let mut parent = ShellView::builder(&cx);
        let activity = parent.include(activity);
        let recommendations = parent.include(recommendations);
        let response = parent
            .finish(
                view! {
                    cx_ref =>
                    <main>
                        (activity)
                        (recommendations)
                    </main>
                }
                .unwrap(),
            )
            .into_response(&cx)
            .unwrap();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(html.matches("<section>").count(), 2);
        assert!(html.contains("activity loading"));
        assert!(html.contains("recommendations loading"));
        assert!(html.contains("activity ready"));
        assert!(html.contains("recommendations ready"));
    }

    #[test]
    fn patch_html_cannot_close_its_script() {
        let patch = super::patch(7, "</script><script>alert('xss')</script>");

        assert!(!patch.contains("</script><script>"));
        assert!(patch.contains(r"\u003c/script\u003e"));
    }
}
