use topcoat::runtime::{Expr, Js, Surrogated, expr};
use topcoat_runtime_coherence::Case;

#[test]
fn different_values_fail_with_both_outcomes_and_source() {
    let case = Case::evaluated(
        "deliberately unequal",
        Expr::new(1.0, Js::source("cx.hydrate(2)")),
    );
    let report = case.check().unwrap_err().to_string();
    assert!(report.contains("deliberately unequal"));
    assert!(report.contains("3ff0000000000000"));
    assert!(report.contains("4000000000000000"));
    assert!(report.contains("cx.hydrate(2)"));
}

#[test]
fn javascript_exception_does_not_match_a_rust_panic() {
    let value: Option<f64> = None;
    let (run, _) = expr!(|| value.unwrap()).into_evaluated_and_js();
    let case = Case::deferred(
        "panic versus exception",
        Expr::new(run, Js::source("() => { throw new TypeError('broken'); }")),
    );
    let report = case.check().unwrap_err().to_string();
    assert!(report.contains("TypeError: broken"));
    assert!(report.contains("Panic"));
}

#[test]
fn invalid_javascript_is_an_execution_failure() {
    let case = Case::evaluated("invalid syntax", Expr::new((), Js::source("const =")));
    assert!(
        case.check()
            .unwrap_err()
            .to_string()
            .contains("SyntaxError")
    );
}

#[test]
fn known_mismatch_cannot_hide_an_unexpected_pass() {
    let case = Case::evaluated(
        "now coherent",
        Expr::new(true, Js::source("cx.hydrate(true)")),
    );
    assert!(
        case.check_known("some_unit")
            .unwrap_err()
            .to_string()
            .contains("now passes")
    );
}

#[test]
fn known_mismatch_cannot_hide_a_different_failure() {
    let case = Case::evaluated(
        "different mismatch",
        Expr::new(false, Js::source("cx.hydrate(true)")),
    );
    assert!(
        case.check_known("some_unit")
            .unwrap_err()
            .to_string()
            .contains("changed")
    );

    let case = Case::evaluated(
        "execution failure",
        Expr::new(true, Js::source("missing_function()")),
    );
    let report = format!("{:#}", case.check_known("some_unit").unwrap_err());
    assert!(report.contains("ReferenceError"));
}

#[test]
fn missing_baseline_is_an_error() {
    let case = Case::evaluated(
        "missing baseline",
        Expr::new(true, Js::source("cx.hydrate(false)")),
    );
    assert!(
        case.check_known("unknown")
            .unwrap_err()
            .to_string()
            .contains("exactly one baseline")
    );
}

#[test]
fn different_async_values_fail_with_both_outcomes() {
    let case = Case::asynchronous(
        "deliberately unequal async values",
        Expr::new(
            async || 1.0.into_surrogate(),
            Js::source("async () => cx.hydrate(2)"),
        ),
    )
    .unwrap();
    let report = case.check().unwrap_err().to_string();
    assert!(report.contains("3ff0000000000000"));
    assert!(report.contains("4000000000000000"));
}

#[test]
fn javascript_rejection_does_not_match_a_rust_panic() {
    let value: Option<f64> = None;
    let (run, _) = expr!(async || value.unwrap()).into_evaluated_and_js();
    let case = Case::asynchronous(
        "panic versus rejection",
        Expr::new(
            run,
            Js::source("async () => { await Promise.resolve(); throw new TypeError('broken'); }"),
        ),
    )
    .unwrap();
    let report = case.check().unwrap_err().to_string();
    assert!(report.contains("TypeError: broken"));
    assert!(report.contains("Panic"));
}

#[test]
fn async_cases_require_a_promise_and_observable_value() {
    for source in ["() => cx.hydrate(1)", "async () => ({ unexpected: true })"] {
        let case = Case::asynchronous(
            "invalid async result",
            Expr::new(async || 1.0.into_surrogate(), Js::source(source)),
        )
        .unwrap();
        let report = case.check().unwrap_err().to_string();
        assert!(report.contains("JavaScript execution failed"));
    }
}
