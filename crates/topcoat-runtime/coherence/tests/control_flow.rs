use topcoat_runtime_coherence::coherent;

#[test]
fn branching() {
    for condition in [false, true] {
        coherent!(if condition { 1.0 } else { 2.0 });
        coherent!(if condition { "yes" } else { "no" });
        coherent!(if condition {
            1.0
        } else if !condition {
            2.0
        } else {
            3.0
        });
        coherent!(direct => if condition { "yes" } else { "no" });
    }
    let missing: Option<f64> = None;
    coherent!(if true { 1.0 } else { missing.unwrap() });
    coherent!(if false { missing.unwrap() } else { 1.0 });
}

#[test]
fn blocks_and_shadowing() {
    let value = 3.0;
    coherent!({
        let value = value + 1.0;
        value * 2.0
    });
    coherent!({
        let value = value + 1.0;
        let value = value * 2.0;
        value
    });
    coherent!({
        let inner = {
            let value = 9.0;
            value + 1.0
        };
        inner + value
    });
    coherent!(direct => { let value = value + 1.0; value * 2.0 });
}

#[test]
#[allow(clippy::unused_unit)] // Unit literals exercise discarded branch values.
fn statement_values_are_discarded() {
    coherent!({
        if true {
            ()
        }
        2.0
    });
    coherent!({
        if true {
            ()
        } else {
            ()
        };
        3.0
    });
    coherent!({
        if true {
            1.0
        } else {
            2.0
        };
        3.0
    });
    coherent!({
        if true {
            1.0
        } else if false {
            2.0
        } else {
            3.0
        };
        4.0
    });
    coherent!({
        {
            ()
        }
        2.0
    });
    coherent!({
        {
            1.0
        }
        3.0
    });
}

#[test]
fn statements_preserve_control_flow() {
    coherent!({
        if true {
            return 1.0;
        }
        2.0
    });
    coherent!({
        if true {
            return 1.0;
        } else {
            2.0
        };
        3.0
    });
    coherent!({
        {
            if true {
                return 1.0;
            }
            2.0;
        }
        3.0
    });
    for condition in [false, true] {
        coherent!({
            loop {
                if condition {
                    break;
                }
                break;
            }
            3.0
        });
        coherent!({
            while true {
                if condition {
                    break;
                }
                break;
            }
            3.0
        });
    }
    coherent!({
        loop {
            if false {
                continue;
            }
            break;
        }
        3.0
    });
    coherent!({
        while true {
            if false {
                continue;
            }
            break;
        }
        3.0
    });
}

#[test]
fn bounded_loops() {
    coherent!({
        while false {}
        3.0
    });
    coherent!({
        loop {
            break;
        }
        3.0
    });
}
