use topcoat_runtime_coherence::coherent;

#[test]
fn captured_tuples() {
    let pair = (1.5, String::from("two"));
    coherent!(pair);
    coherent!(pair.clone());
    coherent!(pair.0);
    coherent!(pair.1.len());
    coherent!(direct => pair.0);
    coherent!(async => pair.0);
    coherent!(async => pair);

    let nested = ((true, 3u8), (-0.0, vec![1i32]), (String::new(),));
    coherent!(nested);
    coherent!(nested.0.1);
    coherent!(nested.1.1.len());
    coherent!(nested.2.0.is_empty());
}

#[test]
fn tuples_in_containers() {
    for value in [None, Some((-0.0, None::<f64>)), Some((1.0, Some(2.0)))] {
        coherent!(value);
        coherent!(value.unwrap().0);
        coherent!(value.unwrap().1.is_some());
    }

    for value in [Ok((1usize, false)), Err((String::from("failed"),))] {
        coherent!(value);
        coherent!(value.unwrap().1);
        coherent!(value.unwrap_err().0);
    }

    let values = vec![(0usize, String::from("a")), (usize::MAX, String::new())];
    coherent!(values.clone());
    coherent!(*values.first().unwrap().0);
    coherent!(values.index(1).1.is_empty());
    coherent!(values.last().unwrap().clone());
    coherent!(values.get(2).is_none());

    let pairs = [((1u8,), 2.5)];
    coherent!(pairs);
    coherent!(*pairs.index(0).0.0);
    coherent!(pairs.first().unwrap().1.clone());
}
