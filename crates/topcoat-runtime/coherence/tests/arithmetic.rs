use topcoat_runtime_coherence::coherent;

const NUMBERS: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    2.0,
    -2.0,
    0.1,
    -0.1,
    f64::EPSILON,
    -f64::EPSILON,
    f64::MIN_POSITIVE,
    f64::from_bits(1),
    -f64::from_bits(1),
    f64::MAX,
    f64::MIN,
    1e-100,
    1e100,
    9_007_199_254_740_992.0,
];

#[test]
fn arithmetic_and_comparison_matrix() {
    for &left in NUMBERS {
        for &right in NUMBERS {
            coherent!(left + right);
            coherent!(left - right);
            coherent!(left * right);
            coherent!(left / right);
            coherent!(left == right);
            coherent!(left != right);
            coherent!(left < right);
            coherent!(left <= right);
            coherent!(left > right);
            coherent!(left >= right);
        }
        coherent!(-left);
        coherent!(direct => left);
    }
}

#[test]
fn precedence_and_grouping() {
    coherent!(1.0 + 2.0 * 3.0);
    coherent!((1.0 + 2.0) * 3.0);
    coherent!(10.0 - 3.0 - 2.0);
    coherent!(10.0 - (3.0 - 2.0));
    coherent!(12.0 / 3.0 / 2.0);
    coherent!(12.0 / (3.0 / 2.0));
    coherent!(-(1.0 + 2.0));
    coherent!(-(-0.0));
    coherent!((1.0 + 2.0) < (2.0 * 3.0));
}

#[test]
fn nonfinite_values_created_inside_expressions() {
    coherent!((0.0 / 0.0) == (0.0 / 0.0));
    coherent!((0.0 / 0.0) != (0.0 / 0.0));
    coherent!((0.0 / 0.0) < 1.0);
    coherent!((0.0 / 0.0) >= 1.0);
    coherent!((1.0 / 0.0) + (-1.0 / 0.0));
    coherent!(0.0 * (1.0 / 0.0));
    coherent!(1.0 / (1.0 / 0.0));
    coherent!(-1.0 / (1.0 / 0.0));
}
