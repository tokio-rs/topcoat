use std::sync::Mutex;

use tokio::sync::broadcast;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::page,
    runtime::{Event, connected, procedure, signal},
    view::{View, emit, live, view},
};

// The HTTP response shows the current messages and finishes. Calling
// connected() makes the browser open a connection and render the page again.
// That render waits for new messages and updates the list in each open tab.
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

/// Stores chat messages and notifies subscribers when a message is added.
pub struct Chat {
    messages: Mutex<Vec<String>>,
    changed: broadcast::Sender<()>,
}

impl Chat {
    fn messages(&self) -> Vec<String> {
        self.messages.lock().unwrap().clone()
    }

    /// Call before reading messages to avoid missing any that arrive
    /// between the read and the subscription.
    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.changed.subscribe()
    }

    fn send(&self, message: String) {
        self.messages.lock().unwrap().push(message);
        // Ignore send errors when there are no subscribers to notify.
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
