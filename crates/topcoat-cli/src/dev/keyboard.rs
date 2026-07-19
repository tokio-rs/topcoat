use std::future::pending;
use std::thread;

use console::{Key, Term};
use tokio::sync::mpsc;

/// Listens for the manual reload key on the terminal.
///
/// Reads single keys from stdin on a background thread and reports each press
/// of `r`, the manual reload shortcut. Active only when attached to an
/// interactive terminal; otherwise [`Self::reload_requested`] never resolves,
/// leaving the event loop driven entirely by file changes.
pub struct Keyboard {
    /// `None` when there is no terminal to read keys from.
    presses: Option<mpsc::UnboundedReceiver<()>>,
}

impl Keyboard {
    /// Start listening for keypresses.
    pub fn start() -> Self {
        let term = Term::stdout();
        if !term.is_term() {
            return Self { presses: None };
        }

        let (tx, presses) = mpsc::unbounded_channel();
        // A detached thread: `read_key` blocks, so it cannot run on the async
        // runtime, and the process exits without waiting for it on shutdown.
        thread::spawn(move || {
            // `read_key` re-raises SIGINT on Ctrl-C, so the dev server's
            // Ctrl-C handler still shuts everything down.
            while let Ok(key) = term.read_key() {
                if matches!(key, Key::Char('r' | 'R')) && tx.send(()).is_err() {
                    break;
                }
            }
        });

        Self {
            presses: Some(presses),
        }
    }

    /// Whether keypresses are being listened for, and so the shortcut is worth
    /// announcing.
    pub fn is_listening(&self) -> bool {
        self.presses.is_some()
    }

    /// Wait until the manual reload key (`r`) is pressed.
    ///
    /// Resolves once per press. Never resolves when there is no terminal, or
    /// once the reader thread has stopped (its stdin closed), so the branch
    /// stays quietly pending rather than spinning the event loop.
    ///
    /// Cancel-safe: a press arriving before cancellation is queued by the
    /// reader thread and reported by the next call.
    pub async fn reload_requested(&mut self) {
        let press = match &mut self.presses {
            Some(presses) => presses.recv().await,
            None => None,
        };
        if press.is_none() {
            pending::<()>().await;
        }
    }
}
