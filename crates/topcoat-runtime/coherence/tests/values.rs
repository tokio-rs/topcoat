use topcoat_runtime_coherence::coherent;

#[test]
fn booleans_and_lazy_values() {
    for left in [false, true] {
        coherent!(!left);
        coherent!(left.then_some(3.0));
        coherent!(left.then(|| "yes"));
        for right in [false, true] {
            coherent!(left == right);
            coherent!(left != right);
        }
    }
    let missing: Option<f64> = None;
    coherent!(false.then(|| missing.unwrap()));
    coherent!(false.then_some(missing.unwrap()));
}

#[test]
fn captured_options() {
    for value in [None, Some(0.0), Some(-0.0), Some(42.0)] {
        coherent!(value);
        coherent!(value.is_some());
        coherent!(value.is_none());
        coherent!(value.unwrap());
        coherent!(value.expect("missing value"));
    }
    for value in [None, Some(None), Some(Some(String::from("hello")))] {
        coherent!(value);
        coherent!(value.is_some());
    }
}

#[test]
fn captured_results() {
    for value in [Ok(0.0), Ok(-0.0), Err(String::from("failed"))] {
        coherent!(value);
        coherent!(value.is_ok());
        coherent!(value.is_err());
        coherent!(value.ok());
        coherent!(value.err());
        coherent!(value.unwrap());
        coherent!(value.unwrap_err());
        coherent!(value.expect("expected success"));
        coherent!(value.expect_err("expected failure"));
    }
}

#[test]
fn unit() {
    coherent!({});
    coherent!({
        let _value = 3.0;
    });
    coherent!(if false {});
}
