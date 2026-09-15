use topcoat_runtime_coherence::coherent;

#[test]
fn arithmetic() {
    coherent!(1.0 + 2.0 * 3.0);
    coherent!(direct => 1.0 + 2.0 * 3.0);
}

#[test]
fn captured_unicode() {
    let text = String::from("\u{0085}\u{1f980}\u{feff}");
    coherent!(text.trim().to_owned());
    coherent!(text.len());
    coherent!(direct => text.len());
}

#[test]
fn special_float_results() {
    coherent!(-0.0);
    coherent!(1.0 / 0.0);
    coherent!(-1.0 / 0.0);
    coherent!(0.0 / 0.0);
}

#[test]
fn panic_is_an_outcome() {
    let value: Option<f64> = None;
    coherent!(value.unwrap());
}
