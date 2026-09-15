use std::cell::Cell;

use topcoat_runtime_coherence::{Awaitable, coherent};

#[test]
fn copies_elements_and_checks_bounds() {
    for values in [vec![], vec![0usize], vec![7, 11, usize::MAX]] {
        for index in [0usize, 1, values.len(), usize::MAX] {
            coherent!(values[index]);
            coherent!(values[index] + 1);
            coherent!(values[index] == 7);
            let slice = values.as_slice();
            coherent!(slice[index]);
        }
    }
    let array = [i128::MIN, -1, i128::MAX];
    coherent!(direct => array[2]);
    coherent!(-array[1]);
    coherent!(array[0] / array[1]);
    coherent!(array[3]);
    let empty: [usize; 0] = [];
    coherent!(empty[0]);
}

#[test]
fn borrows_for_methods_and_clones_explicitly() {
    let names = vec![String::from("first"), String::from("last")];
    coherent!(names[0].len());
    coherent!(names[1].clone());
    coherent!(names[1].to_owned());
    coherent!(names[0] == names[1]);
    coherent!(names[1].trim().to_owned());
    coherent!(direct => names[0].clone());
    let slice = &names[1..];
    coherent!(slice[0].to_owned());
}

#[test]
fn nested_indexes_and_expression_precedence() {
    let nested = vec![vec![2usize, 3], vec![5, 7]];
    coherent!(nested[1][0]);
    coherent!(nested[0].len());
    coherent!(nested[1].clone());
    coherent!(nested[0][1] * nested[1][0] + 1);
    coherent!((nested[1][1] + 1) * 2);
    coherent!(nested[0][nested[0].len() - 1]);
    coherent!((if true { nested.clone() } else { nested.to_vec() })[1][0]);
    coherent!(nested.to_vec()[1][0]);
    let arrays = [[1usize, 2], [3, 4]];
    coherent!(arrays[1]);
    coherent!(arrays[1][0]);
    coherent!({
        let row = arrays[1];
        row[0] + row[1]
    });
}

#[test]
fn evaluates_base_then_index_exactly_once() {
    let trace = Cell::new(0u32);
    let values = vec![10usize, 20];
    let index = 1usize;
    coherent!({
        let selected = raw!(
            "((globalThis.__indexTrace = (globalThis.__indexTrace ?? 0) * 10 + 1), ${values})",
            {
                trace.set(trace.get() * 10 + 1);
                values.clone()
            }
        )[raw!(
            "((globalThis.__indexTrace = globalThis.__indexTrace * 10 + 2), ${index})",
            {
                trace.set(trace.get() * 10 + 2);
                index
            }
        )];
        raw!(
            "(() => { if (globalThis.__indexTrace !== 12) throw new Error('index evaluation order'); return ${selected}; })()",
            {
                assert_eq!(trace.get(), 12);
                selected
            }
        )
    });
}

#[test]
fn indexes_after_await() {
    let values = Awaitable::ready(vec![42usize]).after_yield();
    coherent!(async => values.await[0]);
}
