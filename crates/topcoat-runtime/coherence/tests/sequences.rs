use topcoat_runtime_coherence::coherent;

#[test]
fn vectors_and_slices() {
    for values in [vec![], vec![0usize], vec![0, 1, usize::MAX]] {
        coherent!(direct => values);
        coherent!(values.clone());
        coherent!(values.len());
        coherent!(values.is_empty());
        coherent!(values.as_slice().len());
        coherent!(values.as_slice().to_vec());
        coherent!(values.as_slice().to_owned());
        coherent!(values.first().is_some());
        coherent!(values.last().is_none());
        coherent!(values.first().unwrap().clone());
        coherent!(*values.last().unwrap());

        let borrowed = &values;
        coherent!(direct => borrowed);
        coherent!(borrowed.len());

        let slice = values.as_slice();
        coherent!(direct => slice);
        coherent!(slice.to_vec());
        coherent!(slice.to_owned());
        coherent!(slice.len());
        coherent!(slice.is_empty());
        coherent!(direct => slice.first());
        coherent!(direct => slice.last());
        for index in [0usize, 1, values.len(), usize::MAX] {
            coherent!(values.get(index).is_some());
            coherent!(values.get(index).is_none());
            coherent!(*values.get(index).unwrap());
            coherent!(values.get(index).unwrap().clone());
            coherent!(slice.get(index).expect("missing").clone());
            coherent!(direct => slice.get(index));
        }
    }
}

#[test]
fn borrowed_strings() {
    let values = vec![
        String::new(),
        String::from("hello"),
        String::from("\u{1f980}"),
    ];
    coherent!(direct => values);
    coherent!(values.get(1).unwrap().to_owned());
    coherent!(values.get(1).unwrap().clone());
    coherent!(values.last().unwrap().len());
    coherent!(values.first().unwrap().is_empty());
    coherent!(values.get(1).clone().unwrap().to_owned());
    let slice = &values[1..];
    coherent!(direct => slice);
    coherent!(slice.to_vec());
    coherent!(direct => slice.first());
    coherent!(direct => slice.clone());
}

#[test]
fn nested_collections_and_options() {
    let values = vec![
        vec![],
        vec![Some(u128::MAX), None, Some(9_007_199_254_740_993)],
    ];
    coherent!(direct => values);
    coherent!(values.clone());
    coherent!(values.as_slice().to_owned());
    coherent!(values.get(0).unwrap().is_empty());
    coherent!(values.get(1).unwrap().get(0).unwrap().clone());
    coherent!(values.get(1).unwrap().get(1).unwrap().clone());
    coherent!(values.get(1).unwrap().last().unwrap().clone().unwrap());
    coherent!(values.get(1).unwrap().clone());
    let optional = Some(values);
    coherent!(optional.clone().unwrap());
}

#[test]
fn asynchronous_reads() {
    let values = vec![true, false];
    coherent!(async => values.get(0).unwrap().clone());
    coherent!(async => values.to_vec());
    let slice = values.as_slice();
    coherent!(async => slice.first());
}

#[test]
fn arrays() {
    fn check<const N: usize>(values: [u128; N]) {
        coherent!(direct => values);
        coherent!(values.len());
        coherent!(values.is_empty());
        coherent!(values.to_vec());
        coherent!(values.to_owned());
        coherent!(values.as_slice().to_vec());
        coherent!(values.first().is_some());
        coherent!(*values.last().unwrap());
        let borrowed = &values;
        coherent!(direct => borrowed);
        coherent!(borrowed.get(0).unwrap().clone());
        let slice = values.as_slice();
        coherent!(direct => slice);
    }
    check([]);
    check([u128::MAX]);
    check([9_007_199_254_740_993; 64]);
    let values = [String::from("first"), String::from("last")];
    coherent!(values.clone());
    coherent!(values.to_owned());
    coherent!(values.last().unwrap().to_owned());
    let nested = [[true, false], [false, true]];
    coherent!(direct => nested);
    coherent!(*nested.last().unwrap().first().unwrap());
}
