use topcoat::runtime::expr;
use topcoat_runtime_coherence::{Case, coherent};

#[test]
fn captured_tuple_field() {
    let pair = (1.5, 2.5);
    Case::evaluated("pair.0", expr!(pair.0)).assert_known("captured_tuple_field");
    coherent!(known "captured_tuple_field" => pair.0);
    coherent!(async known "captured_tuple_field" => pair.0);
}

#[test]
fn captured_tuple_value() {
    let pair = (1.5, 2.5);
    Case::evaluated("pair", expr!(pair)).assert_known("captured_tuple_value");
    coherent!(known "captured_tuple_value" => pair);
    coherent!(async known "captured_tuple_value" => pair);
}

#[test]
fn captured_tuple_with_option() {
    let pair = (1.0, Some(2.0));
    coherent!(known "captured_tuple_with_option" => pair);
}

#[test]
fn captured_nested_tuple_field() {
    let nested = ((1.5, 2.5), true);
    coherent!(known "captured_tuple_field" => nested.0.0);
}

#[test]
fn captured_nan() {
    let value = f64::NAN;
    coherent!(known "captured_nan" => value);
}

#[test]
fn captured_infinity() {
    let value = f64::INFINITY;
    coherent!(known "captured_infinity" => value);
}

#[test]
fn some_unit() {
    coherent!(known "some_unit" => true.then_some({}).is_some());
}

#[test]
fn return_inside_if() {
    coherent!(known "return_inside_if" => { if true { return 1.0; } 2.0 });
}

#[test]
fn break_inside_if_inside_loop() {
    coherent!(known "break_inside_if" => {
        loop {
            if true { break; }
            break;
        }
        3.0
    });
}

#[test]
fn break_inside_if_inside_while() {
    coherent!(known "break_inside_if" => {
        while true {
            if true { break; }
            break;
        }
        3.0
    });
}

#[test]
fn continue_inside_if_inside_loop() {
    // The branch is untaken so Rust terminates. JavaScript rejects the jump
    // during compilation, regardless of the condition's value.
    coherent!(known "continue_inside_if" => {
        loop {
            if false { continue; }
            break;
        }
        3.0
    });
}

#[test]
fn continue_inside_if_inside_while() {
    coherent!(known "continue_inside_if" => {
        while true {
            if false { continue; }
            break;
        }
        3.0
    });
}
