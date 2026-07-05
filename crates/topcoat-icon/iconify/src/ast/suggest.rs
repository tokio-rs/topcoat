use std::fmt::Write as _;

/// Renders the near misses of `target` among `candidates` as a
/// `did you mean ...?` fragment for an error message, or `None` when nothing
/// comes close.
pub(crate) fn did_you_mean<'a>(
    target: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let suggestions = suggestions(target, candidates);
    let (last, head) = suggestions.split_last()?;

    let mut message = String::from("did you mean ");
    for suggestion in head {
        let _ = write!(message, "`{suggestion}`, ");
    }
    if !head.is_empty() {
        message.push_str("or ");
    }
    let _ = write!(message, "`{last}`?");
    Some(message)
}

/// Up to three candidates within a third of `target`'s length in edit
/// distance, closest first.
fn suggestions<'a>(target: &str, candidates: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    let mut near: Vec<(usize, &str)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let distance = edit_distance(target, candidate);
            let max = target.len().max(candidate.len()).div_ceil(3);
            (distance <= max).then_some((distance, candidate))
        })
        .collect();
    near.sort_unstable_by_key(|&(distance, candidate)| (distance, candidate));
    near.truncate(3);
    near.into_iter().map(|(_, candidate)| candidate).collect()
}

/// The Levenshtein distance between `a` and `b`.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();

    // A single-row edit matrix: after processing `a[..i]`, `row[j]` is the
    // distance between `a[..i]` and `b[..j]`.
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let substitute = diagonal + usize::from(ca != cb);
            diagonal = row[j + 1];
            row[j + 1] = substitute.min(row[j] + 1).min(row[j + 1] + 1);
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_counts_edits() {
        assert_eq!(edit_distance("trash", "trash"), 0);
        assert_eq!(edit_distance("trash", "tras"), 1);
        assert_eq!(edit_distance("trash", "rash"), 1);
        assert_eq!(edit_distance("trash", "crash"), 1);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
    }

    #[test]
    fn suggestions_rank_near_misses() {
        let candidates = ["delete", "delta", "delete-2", "pencil"];
        assert_eq!(
            suggestions("delet", candidates),
            vec!["delete", "delta", "delete-2"]
        );
        assert_eq!(suggestions("eraser", candidates), Vec::<&str>::new());
    }

    #[test]
    fn did_you_mean_lists_up_to_three() {
        assert_eq!(
            did_you_mean("delet", ["delete"]),
            Some("did you mean `delete`?".to_owned())
        );
        assert_eq!(
            did_you_mean("delet", ["delete", "delta"]),
            Some("did you mean `delete`, or `delta`?".to_owned())
        );
        assert_eq!(did_you_mean("eraser", ["delete"]), None);
    }
}
