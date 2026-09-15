use topcoat_runtime_coherence::coherent;

#[test]
fn literals() {
    coherent!(direct => 42);
    coherent!(1 + 2 * 3);
    coherent!(7 % 3);
    coherent!(0xffu8);
    coherent!(0o777u16);
    coherent!(0b1010u32);
    coherent!(9_007_199_254_740_993u64);
    coherent!(340282366920938463463374607431768211455u128);
    coherent!(42usize);
    coherent!(-128i8);
    coherent!(-32768i16);
    coherent!(-2147483648i32);
    coherent!(-9223372036854775808i64);
    coherent!(-170141183460469231731687303715884105728i128);
    coherent!(-1isize);
    coherent!(-(128i8));
    coherent!(-(-128i8));
    coherent!(255u8 + 1u8);
    let value = 4usize;
    coherent!(1 + value + 2 + value);
    coherent!({
        let value = 1;
        value + 2
    });
    coherent!(async => 42u128);
}

macro_rules! integer_cases {
    ($integer:ident) => {
        mod $integer {
            use super::*;

            #[test]
            fn arithmetic_and_comparisons() {
                let values: &[$integer] = &[
                    $integer::MIN, $integer::MIN + 1, 0, 1, 2,
                    $integer::MAX / 2, $integer::MAX - 1, $integer::MAX,
                ];
                for &left in values {
                    coherent!(direct => left);
                    let borrowed = &left;
                    coherent!(direct => borrowed);
                    let nested = Some(Some(left));
                    coherent!(nested);
                    for &right in values {
                        coherent!(left + right);
                        coherent!(left - right);
                        coherent!(left * right);
                        coherent!(left / right);
                        coherent!(left % right);
                        coherent!(left == right);
                        coherent!(left != right);
                        coherent!(left < right);
                        coherent!(left <= right);
                        coherent!(left > right);
                        coherent!(left >= right);
                    }
                }
            }

            #[test]
            fn preserves_values_above_javascript_number_precision() {
                for value in [
                    9_007_199_254_740_991_u128,
                    9_007_199_254_740_992,
                    9_007_199_254_740_993,
                    18_446_744_073_709_551_617,
                ] {
                    if let Ok(value) = $integer::try_from(value) {
                        let one: $integer = 1;
                        coherent!(direct => value);
                        coherent!((value + one) - value);
                        coherent!(value * value);
                    }
                }
            }
        }
    };
}

integer_cases!(u8);
integer_cases!(u16);
integer_cases!(u32);
integer_cases!(u64);
integer_cases!(u128);
integer_cases!(usize);
integer_cases!(i8);
integer_cases!(i16);
integer_cases!(i32);
integer_cases!(i64);
integer_cases!(i128);
integer_cases!(isize);

macro_rules! signed_cases {
    ($integer:ident, $test:ident) => {
        #[test]
        fn $test() {
            let negative_one: $integer = -1;
            let three: $integer = 3;
            for value in [$integer::MIN, $integer::MIN + 1, -7, -1, 0, 1, $integer::MAX] {
                coherent!(-value);
                coherent!(value / negative_one);
                coherent!(value % negative_one);
                coherent!(value / three);
                coherent!(value % three);
            }
        }
    };
}

signed_cases!(i8, signed_i8);
signed_cases!(i16, signed_i16);
signed_cases!(i32, signed_i32);
signed_cases!(i64, signed_i64);
signed_cases!(i128, signed_i128);
signed_cases!(isize, signed_isize);
