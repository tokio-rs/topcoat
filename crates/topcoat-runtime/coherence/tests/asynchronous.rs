use topcoat_runtime_coherence::{Awaitable, coherent};

#[test]
fn async_closures_without_await() {
    coherent!(async => 1.0 + 2.0);
    coherent!(async => true);
    coherent!(async => "hello");
    coherent!(async => {});
}

#[test]
fn ready_and_yielding_values() {
    for yielding in [false, true] {
        for value in [-42.0, -0.0, 0.0, 1.0, f64::MIN_POSITIVE, f64::MAX] {
            let future = Awaitable::ready(value);
            let future = if yielding {
                future.after_yield()
            } else {
                future
            };
            coherent!(async => future.await);
            coherent!(async => future.await + 1.0);
        }
        for value in [false, true] {
            let future = Awaitable::ready(value);
            let future = if yielding {
                future.after_yield()
            } else {
                future
            };
            coherent!(async => future.await);
            coherent!(async => !future.await);
            coherent!(async => future.await.then_some(3.0));
        }
        for value in ["", "hello", "\u{0085}\u{1f980}\u{feff}"] {
            let future = Awaitable::ready(value.to_owned());
            let future = if yielding {
                future.after_yield()
            } else {
                future
            };
            coherent!(async => future.await);
            coherent!(async => future.await.trim().len());
        }
        let future = Awaitable::ready(());
        let future = if yielding {
            future.after_yield()
        } else {
            future
        };
        coherent!(async => future.await);
    }
}

#[test]
fn sequential_awaits_and_nested_blocks() {
    let first = Awaitable::ready(3.0).after_yield();
    let second = Awaitable::ready(4.0).after_yield();
    coherent!(async => first.await + second.await);
    coherent!(async => {
        let value = first.await;
        let inner = {
            let value = value + second.await;
            value * 2.0
        };
        inner + value
    });
}

#[test]
fn awaited_options_and_results() {
    for value in [None, Some(String::new()), Some(String::from("hello"))] {
        let future = Awaitable::ready(value).after_yield();
        coherent!(async => future.await);
        coherent!(async => future.await.is_some());
        coherent!(async => future.await.unwrap());
    }
    for value in [Ok(-0.0), Ok(3.0), Err(String::from("failed"))] {
        let future = Awaitable::ready(value).after_yield();
        coherent!(async => future.await);
        coherent!(async => future.await.is_ok());
        coherent!(async => future.await.unwrap());
        coherent!(async => future.await.unwrap_err());
    }
}

#[test]
fn awaited_conditions_and_branches() {
    for condition in [false, true] {
        let condition = Awaitable::ready(condition).after_yield();
        let left = Awaitable::ready(1.0).after_yield();
        let right = Awaitable::ready(2.0).after_yield();
        coherent!(async => if condition.await { left.await } else { right.await });
        let other = Awaitable::ready(true).after_yield();
        coherent!(async => if condition.await {
            left.await
        } else if other.await {
            right.await
        } else {
            3.0
        });
    }
}

#[test]
fn unused_futures_and_untaken_branches_are_lazy() {
    let failure = Awaitable::<f64>::panicking("must not be polled").after_yield();
    coherent!(async => {
        let _unused = failure;
        3.0
    });
    coherent!(async => if true { 1.0 } else { failure.await });
    coherent!(async => if false { failure.await } else { 1.0 });
}

#[test]
fn panics_before_during_and_after_await() {
    let missing: Option<f64> = None;
    let future = Awaitable::ready(3.0).after_yield();
    coherent!(async => {
        let value = missing.unwrap();
        value + future.await
    });
    coherent!(async => {
        let value = future.await;
        value + missing.unwrap()
    });
    let failure = Awaitable::<f64>::panicking("fixture panic");
    coherent!(async => failure.await);
    let failure = failure.after_yield();
    coherent!(async => failure.await);
}

#[test]
fn known_control_flow_mismatches_after_await() {
    let condition = Awaitable::ready(true).after_yield();
    coherent!(async known "return_inside_if" => {
        if condition.await { return 1.0; }
        2.0
    });
    coherent!(async known "break_inside_if" => {
        loop {
            if condition.await { break; }
            break;
        }
        3.0
    });
    let condition = Awaitable::ready(false).after_yield();
    coherent!(async known "continue_inside_if" => {
        let condition = condition.await;
        loop {
            if condition { continue; }
            break;
        }
        3.0
    });
}
