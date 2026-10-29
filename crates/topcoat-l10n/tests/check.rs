//! Checks a test binary with deliberate translation errors in its `t!` calls and Fluent
//! files.

#![cfg(feature = "check")]

use std::path::Path;

use topcoat::l10n::fluent::messages;
use topcoat_l10n::{Check, Locale, NamespaceCheck, Severity, Translation, locale};

messages!("./check/{locale}.ftl");
const MESSAGES_LINE: u32 = line!() - 1;

/// Translation calls used by the test, paired with their line numbers.
fn calls() -> Vec<(u32, Translation<'static>)> {
    vec![
        (line!(), t!(title)),
        // Does not pass `name`.
        (line!(), t!(greeting)),
        // Passes `extra`, which no locale uses.
        (line!(), t!(greeting, name: "Ada", extra: 1)),
        // German lacks the attribute.
        (line!(), t!(search.placeholder)),
        // A typo of `title`.
        (line!(), t!(titel)),
    ]
}

fn line(index: usize) -> u32 {
    calls()[index].0
}

fn check() -> NamespaceCheck {
    let binary = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    Check::binary(&binary)
        .namespaces
        .into_iter()
        .find(|namespace| namespace.pattern == "./check/{locale}.ftl")
        .expect("the binary has the namespace")
}

/// Lists each problem's severity and line number for the file `name`.
fn problems_in(check: &NamespaceCheck, name: &str) -> Vec<(Severity, u32)> {
    check
        .problems
        .iter()
        .filter(|problem| problem.location.file.file_name() == Some(name.as_ref()))
        .map(|problem| (problem.severity, problem.location.line))
        .collect()
}

#[test]
fn finds_the_namespace_where_it_is_invoked() {
    let check = check();
    assert_eq!(
        check.location.file,
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/check.rs")
    );
    assert_eq!(check.location.line, MESSAGES_LINE);
    assert_eq!(check.location.column, Some(1));
}

#[test]
fn a_locale_covers_the_keys_that_it_or_its_parents_have() {
    let coverage: Vec<(Locale, Vec<String>)> = check()
        .coverage
        .into_iter()
        .map(|coverage| (coverage.locale, coverage.missing))
        .collect();
    let missing = |keys: &[&str]| keys.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(
        coverage,
        [
            (locale!("de"), missing(&["search.placeholder", "titel"])),
            // Takes `greeting` and `search` from `de`.
            (locale!("de-CH"), missing(&["search.placeholder", "titel"])),
            (locale!("en"), missing(&["titel"])),
        ]
    );
}

#[test]
fn reports_problems_at_the_calls() {
    let mut problems = problems_in(&check(), "check.rs");
    problems.sort_unstable();
    let mut expected = vec![
        (Severity::Error, line(1)),
        (Severity::Warning, line(2)),
        (Severity::Error, line(3)),
        (Severity::Error, line(4)),
    ];
    expected.sort_unstable();
    assert_eq!(problems, expected);
}

#[test]
fn suggests_a_key_for_a_typo() {
    let check = check();
    let typo = check
        .problems
        .iter()
        .find(|problem| problem.location.line == line(4))
        .unwrap();
    assert!(typo.message.contains("`title`"), "{}", typo.message);
}

#[test]
fn reports_unknown_references_and_unused_messages_in_the_files() {
    let check = check();
    assert_eq!(
        problems_in(&check, "en.ftl"),
        [
            // `-brand` and `other` are not defined.
            (Severity::Error, 6),
            (Severity::Error, 6),
            (Severity::Warning, 5),
            (Severity::Warning, 6),
        ]
    );
}

#[test]
fn checks_inherited_messages_against_the_locale_that_takes_them() {
    let check = check();
    assert_eq!(
        problems_in(&check, "de.ftl"),
        [
            // `about` works in `de`, but not in `de-CH`.
            (Severity::Error, 7),
            // `nowhere` is not defined, which is reported once, not again
            // for `de-CH`.
            (Severity::Error, 8),
            (Severity::Warning, 7),
            (Severity::Warning, 8),
        ]
    );
    assert_eq!(problems_in(&check, "de-CH.ftl"), []);
}
