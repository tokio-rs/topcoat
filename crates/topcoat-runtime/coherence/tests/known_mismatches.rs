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
