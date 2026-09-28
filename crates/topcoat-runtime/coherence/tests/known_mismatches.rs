use topcoat_runtime_coherence::coherent;

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
