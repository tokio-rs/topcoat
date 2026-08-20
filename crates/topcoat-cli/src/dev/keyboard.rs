use std::{future::pending, thread};

use console::{Key, Term, style};
use tokio::sync::mpsc;

/// Owns the dev server's terminal input.
///
/// Reads every keypress on a single background thread, so nothing else ever
/// contends with it for stdin, and dispatches keys to whichever listener is
/// currently interested: the manual-reload shortcut, or an in-flight
/// [`confirm`](Self::confirm) prompt.
pub struct Keyboard {
    /// `None` when there is no terminal to read keys from.
    keys: Option<mpsc::UnboundedReceiver<Key>>,
}

impl Keyboard {
    /// Start listening for keypresses.
    pub fn start() -> Self {
        let term = Term::stdout();
        if !term.is_term() {
            return Self { keys: None };
        }

        let (tx, keys) = mpsc::unbounded_channel();
        // A detached thread: `read_key` blocks, so it cannot run on the async
        // runtime, and the process exits without waiting for it on shutdown.
        thread::spawn(move || {
            // `read_key` re-raises SIGINT on Ctrl-C, so the dev server's
            // Ctrl-C handler still shuts everything down.
            while let Ok(key) = term.read_key() {
                if tx.send(key).is_err() {
                    break;
                }
            }
        });

        Self { keys: Some(keys) }
    }

    /// Whether keypresses are being listened for, and so the shortcut is worth
    /// announcing.
    pub fn is_listening(&self) -> bool {
        self.keys.is_some()
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
        loop {
            let key = match &mut self.keys {
                Some(keys) => keys.recv().await,
                None => None,
            };
            match key {
                Some(Key::Char('r' | 'R')) => return,
                Some(_) => {}
                None => return pending::<()>().await,
            }
        }
    }

    /// Print `prompt` and wait for a yes/no answer: `y`/`Y`/Enter is yes,
    /// `n`/`N` is no, and any other key is ignored.
    pub async fn confirm(&mut self, prompt: &str) -> bool {
        let Some(keys) = &mut self.keys else {
            return true;
        };

        eprint!("{prompt}");

        let answer = loop {
            match keys.recv().await {
                Some(Key::Enter | Key::Char('y' | 'Y')) => break true,
                Some(Key::Char('n' | 'N')) | None => break false,
                Some(_) => {}
            }
        };

        let echo = if answer {
            style("y").for_stderr().green().bold()
        } else {
            style("n").for_stderr().red().bold()
        };
        eprintln!("{echo}");
        eprintln!();

        answer
    }
}
