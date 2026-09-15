use std::{
    sync::{Once, mpsc},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};

/// A JavaScript engine that gives each evaluation a fresh global context.
pub struct Engine {
    isolate: v8::OwnedIsolate,
    timeout: Duration,
}

impl Engine {
    #[must_use]
    pub fn new(timeout: Duration) -> Self {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });

        Self {
            isolate: v8::Isolate::new(Default::default()),
            timeout,
        }
    }

    /// Evaluates a script whose result must be a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the script throws, exceeds its deadline, or returns
    /// anything other than a string.
    ///
    /// # Panics
    ///
    /// Panics if the watchdog thread panics.
    pub fn evaluate(&mut self, source: &str) -> Result<String> {
        let handle = self.isolate.thread_safe_handle();
        let timeout = self.timeout;
        let (done, receiver) = mpsc::channel();
        let watchdog = thread::spawn(move || {
            if receiver.recv_timeout(timeout) == Err(mpsc::RecvTimeoutError::Timeout) {
                handle.terminate_execution();
                true
            } else {
                false
            }
        });

        let result = self.run(source);
        let _ = done.send(());
        let timed_out = watchdog.join().expect("JavaScript watchdog panicked");
        // Join before resetting termination so a late watchdog cannot interrupt
        // the next evaluation on this isolate.
        self.isolate.cancel_terminate_execution();
        if timed_out {
            bail!("JavaScript evaluation exceeded {timeout:?}");
        }
        result
    }

    fn run(&mut self, source: &str) -> Result<String> {
        let scope = std::pin::pin!(v8::HandleScope::new(&mut self.isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();

        let source = v8::String::new(scope, source).context("JavaScript source is too large")?;
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        let Some(result) = result else {
            let exception = scope
                .exception()
                .map(|exception| exception.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "execution terminated without an exception".to_owned());
            bail!("JavaScript execution failed: {exception}");
        };
        let result = v8::Local::<v8::String>::try_from(result)
            .map_err(|_| anyhow::anyhow!("JavaScript script must return a string"))?;
        Ok(result.to_rust_string_lossy(scope))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_javascript() {
        let mut engine = Engine::new(Duration::from_secs(5));
        assert_eq!(engine.evaluate("String(1 + 2 * 3)").unwrap(), "7");
    }

    #[test]
    fn reports_syntax_errors_and_exceptions() {
        let mut engine = Engine::new(Duration::from_secs(5));
        assert!(engine.evaluate("const =").unwrap_err().to_string().contains("SyntaxError"));
        assert!(
            engine.evaluate("throw new TypeError('broken')")
                .unwrap_err().to_string().contains("TypeError: broken")
        );
        assert!(engine.evaluate("42").is_err());
    }

    #[test]
    fn globals_do_not_leak_between_evaluations() {
        let mut engine = Engine::new(Duration::from_secs(5));
        engine.evaluate("globalThis.leaked = true; ''").unwrap();
        assert_eq!(engine.evaluate("typeof leaked").unwrap(), "undefined");
    }

    #[test]
    fn terminates_infinite_loops_and_recovers() {
        let mut engine = Engine::new(Duration::from_millis(100));
        assert!(engine.evaluate("for (;;) {}").unwrap_err().to_string().contains("exceeded"));
        assert_eq!(engine.evaluate("'recovered'").unwrap(), "recovered");
    }
}
