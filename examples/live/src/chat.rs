use std::sync::Mutex;

use tokio::sync::broadcast;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::page,
    runtime::{Event, connected, procedure, signal},
    view::{View, emit, live, view},
};

// Over HTTP, the region shows the messages once and the page finishes. The
// region asks for a connection, so the browser then opens one and renders
// the page again over it. That render keeps its subscription and emits the
// list again each time a message arrives, in every tab showing the page.
#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <h1>"Chat"</h1>

        (live! {
            let chat = app_context::<Chat>(cx);
            let mut changed = chat.subscribe();
            loop {
                let token = emit! {
                    <ul>
                        for message in chat.messages() {
                            <li>(message)</li>
                        }
                    </ul>
                }?;
                if !connected(cx) {
                    break Ok(token);
                }
                changed.recv().await.ok();
            }
        })

        let draft = signal(cx, String::new);
        <input :value=$(draft.get()) @input=$(|e: Event| draft.set(e.target.value))>
        <button
            @click=$(async |_e| {
                send(draft.get()).await;
                draft.set("".to_owned());
            })
        >
            "Send"
        </button>

        <p>"Hint: open this example in two tabs."</p>
    })
}

#[procedure]
pub async fn send(cx: &Cx, message: String) -> Result<bool> {
    app_context::<Chat>(cx).send(message);
    Ok(true)
}

/// The messages sent so far, and a channel that announces each new one.
pub struct Chat {
    messages: Mutex<Vec<String>>,
    changed: broadcast::Sender<()>,
}

impl Chat {
    fn messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }

    /// Subscribe before reading the messages, so one sent in between is
    /// not missed.
    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }

    fn send(&self, message: String) {
        self.messages.lock().unwrap().push(message);
        // Fails only when no page is subscribed, so there is nothing to wake.
        let _ = self.changed.send(());
    }
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
            changed: broadcast::channel(16).0,
        }
    }
}
