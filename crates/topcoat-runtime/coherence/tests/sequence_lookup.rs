use topcoat::runtime::{Surrogated, expr, record};
use topcoat_runtime_coherence::{Case, coherent};

#[test]
fn repeated_positions_are_relative_to_the_remaining_suffix() {
    let values = [10u32, 20, 20, 30];
    let actual = expr!({
        let mut iter = values.iter();
        let first = iter.position(|value| value == 20u32);
        let second = iter.position(|value| value == 20u32);
        let third = iter.position(|value| value == 20u32);
        let exhausted = iter.position(|_value| true);
        (first, second, third, exhausted)
    });
    assert_eq!(
        actual.clone().into_evaluated_and_js().0,
        (Some(1), Some(0), None, None)
    );
    Case::evaluated("remaining suffix positions", actual).assert();
}

#[test]
fn empty_missing_first_last_and_duplicate_matches() {
    for values in [vec![], vec![10u32], vec![10, 20, 20, 30]] {
        let slice = values.as_slice();
        for target in [10u32, 20, 30, 40] {
            let actual = expr!(slice.iter().position(|value| value == target));
            assert_eq!(
                actual.clone().into_evaluated_and_js().0,
                slice.iter().position(|value| *value == target)
            );
            Case::evaluated("borrowed scalar lookup", actual).assert();
            coherent!(values.iter().position(|value| value == target));
        }
    }
    let values = [false, true, false];
    coherent!(values.iter().position(|value| *value));
    let tuples = [(1u32, "a".to_owned()), (2, "b".to_owned())];
    coherent!(tuples.iter().position(|value| value.0 == 2u32));
    coherent!(tuples.iter().position(|value| value.1 == "b"));
}

#[test]
fn borrowed_string_fields_and_subslices() {
    let values = [
        "outside".to_owned(),
        String::new(),
        "\u{1f980}".to_owned(),
        "end".to_owned(),
    ];
    let slice = &values[1..];
    for target in ["", "\u{1f980}", "end", "outside"] {
        let actual = expr!(slice.iter().position(|value| value == target));
        assert_eq!(
            actual.clone().into_evaluated_and_js().0,
            slice.iter().position(|value| value == target)
        );
        Case::evaluated("string subslice lookup", actual).assert();
    }
}

#[record]
struct Choice {
    value: String,
    rank: u32,
}

#[test]
fn borrowed_records_need_no_clone() {
    let choices = [
        Choice {
            value: "a".to_owned(),
            rank: 1,
        },
        Choice {
            value: "b".to_owned(),
            rank: 2,
        },
    ];
    let rows = choices.as_slice();
    let value = "b".to_owned();
    coherent!(rows.iter().position(|row| row.value == value));
    coherent!(rows.iter().position(|row| row.rank == 2u32));
    coherent!(rows.iter().position(|row| row.value.is_empty()));
}

#[test]
fn predicates_stop_at_matches_and_propagate_reachable_panics() {
    let values = [Some(false), Some(true), None];
    coherent!(values.iter().position(|value| value.clone().unwrap()));
    let values = [Some(false), None, Some(true)];
    coherent!(values.iter().position(|value| value.clone().unwrap()));
    let empty: [Option<bool>; 0] = [];
    coherent!(empty.iter().position(|_value| None::<bool>.unwrap()));
}

#[test]
fn large_record_lookups_keep_the_same_generated_code() {
    let mut code = None;
    for count in [0, 1, 200, 800, 10_000, 50_000] {
        let choices: Vec<_> = (0..count)
            .map(|i| Choice {
                value: if i + 1 == count {
                    "target".to_owned()
                } else {
                    format!("row-{i}")
                },
                rank: i,
            })
            .collect();
        let rows = choices.as_slice();
        let actual = expr!(rows.iter().position(|row| row.value == "target"));
        let (result, js) = actual.clone().into_evaluated_and_js();
        assert_eq!(result, (count as usize).checked_sub(1));
        let payload = serde_json::to_string(&rows.into_surrogate()).unwrap();
        let capture = format!("cx.hydrate({payload})");
        let source = js.to_source();
        assert_eq!(source.matches(&capture).count(), 1);
        let normalized = source.replace(&capture, "ROWS");
        if let Some(previous) = &code {
            assert_eq!(&normalized, previous);
        } else {
            code = Some(normalized);
        }
        Case::evaluated("large last-row lookup", actual).assert();
        let absent = expr!(rows.iter().position(|row| row.value == "missing"));
        assert_eq!(absent.clone().into_evaluated_and_js().0, None);
        Case::evaluated("large missing-row lookup", absent).assert();
    }
}
