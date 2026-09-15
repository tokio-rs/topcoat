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
            isolate: v8::Isolate::new(v8::CreateParams::default()),
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
        let context = v8::Context::new(scope, v8::ContextOptions::default());
        let scope = &mut v8::ContextScope::new(scope, context);
        let scope = std::pin::pin!(v8::TryCatch::new(scope));
        let scope = &mut scope.init();

        Self::install_text_encoder(scope)?;
        let source = format!(
            "globalThis.TextEncoder = class {{ encode(input = '') {{ \
             return __coherence_encode_utf8(String(input)); }} }};\n{source}"
        );
        let source = v8::String::new(scope, &source).context("JavaScript source is too large")?;
        let result = v8::Script::compile(scope, source, None).and_then(|script| script.run(scope));
        let Some(result) = result else {
            let exception = scope.exception().map_or_else(
                || "execution terminated without an exception".to_owned(),
                |exception| exception.to_rust_string_lossy(scope),
            );
            bail!("JavaScript execution failed: {exception}");
        };
        let result = v8::Local::<v8::String>::try_from(result)
            .map_err(|_| anyhow::anyhow!("JavaScript script must return a string"))?;
        Ok(result.to_rust_string_lossy(scope))
    }

    fn install_text_encoder(scope: &mut v8::PinScope<'_, '_>) -> Result<()> {
        // TextEncoder is a host API, not part of V8. Only its encode method is
        // needed by the runtime. String conversion replaces lone surrogates,
        // matching the Web API's conversion to Unicode scalar values.
        let encode = v8::Function::new(
            scope,
            |scope: &mut v8::PinScope,
             args: v8::FunctionCallbackArguments,
             mut result: v8::ReturnValue| {
                let Some(text) = args.get(0).to_string(scope) else {
                    return;
                };
                let bytes = text.to_rust_string_lossy(scope).into_bytes();
                let length = bytes.len();
                let store = v8::ArrayBuffer::new_backing_store_from_vec(bytes).make_shared();
                let buffer = v8::ArrayBuffer::with_backing_store(scope, &store);
                if let Some(array) = v8::Uint8Array::new(scope, buffer, 0, length) {
                    result.set(array.into());
                }
            },
        )
        .context("failed to create UTF-8 host function")?;
        let name = v8::String::new(scope, "__coherence_encode_utf8")
            .context("failed to name UTF-8 host function")?;
        let global = scope.get_current_context().global(scope);
        anyhow::ensure!(
            global.set(scope, name.into(), encode.into()) == Some(true),
            "failed to install UTF-8 host function"
        );
        Ok(())
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
        assert!(
            engine
                .evaluate("const =")
                .unwrap_err()
                .to_string()
                .contains("SyntaxError")
        );
        assert!(
            engine
                .evaluate("throw new TypeError('broken')")
                .unwrap_err()
                .to_string()
                .contains("TypeError: broken")
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
    fn text_encoder_handles_unicode_and_lone_surrogates() {
        let mut engine = Engine::new(Duration::from_secs(5));
        assert_eq!(
            engine
                .evaluate(r"String(new TextEncoder().encode('\u{1f980}\ud800'))")
                .unwrap(),
            "240,159,166,128,239,191,189",
        );
    }

    #[test]
    fn terminates_infinite_loops_and_recovers() {
        let mut engine = Engine::new(Duration::from_millis(100));
        assert!(
            engine
                .evaluate("for (;;) {}")
                .unwrap_err()
                .to_string()
                .contains("exceeded")
        );
        assert_eq!(engine.evaluate("'recovered'").unwrap(), "recovered");
    }
}
