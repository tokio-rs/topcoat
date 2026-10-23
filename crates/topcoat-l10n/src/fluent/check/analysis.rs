use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    path::PathBuf,
};

use fluent_syntax::ast::{Attribute, Entry, InlineExpression, Pattern};

use super::{Coverage, Location, ParsedFile, Problem, Severity, Sources, visit_expressions};
use crate::{Locale, RawTranslation, lookup_indices, normalize_for_lookup};

/// Checks the files of `sources` against `calls`, the `t!` calls of their
/// namespace. Adds the problems to `problems` and returns the coverage of
/// each locale.
pub(super) fn analyze(
    sources: &Sources,
    calls: &[RawTranslation<'_>],
    problems: &mut Vec<Problem>,
) -> Vec<Coverage> {
    let files: Vec<Vec<ParsedFile<'_>>> = sources
        .locales
        .iter()
        .map(|(_, files)| {
            let files = files
                .iter()
                .map(|(path, text)| ParsedFile::parse(path, text));
            files.collect()
        })
        .collect();
    let locales: Vec<&Locale> = sources.locales.iter().map(|(locale, _)| locale).collect();
    let bundles = Bundle::of_locales(&locales, &files);

    check_calls(calls, &locales, &bundles, problems);
    check_references(&files, &bundles, problems);
    check_unused(calls, &files, problems);
    coverage(calls, &locales, &bundles)
}

/// The messages and terms that one locale has: its own, then the ones it takes
/// from its parents.
#[derive(Default)]
struct Bundle<'s> {
    messages: HashMap<&'s str, Definition<'s>>,
    terms: HashMap<&'s str, Definition<'s>>,
}

/// The value and the attributes of a message or term.
#[derive(Clone, Copy)]
struct Definition<'s> {
    value: Option<&'s Pattern<&'s str>>,
    attributes: &'s [Attribute<&'s str>],
}

impl<'s> Bundle<'s> {
    /// Returns the bundle of each locale, made like the bundles of a
    /// [`Catalog`](crate::Catalog).
    fn of_locales(locales: &[&Locale], files: &'s [Vec<ParsedFile<'s>>]) -> Vec<Self> {
        let own: Vec<_> = files.iter().map(|files| Self::own(files)).collect();
        let normalized: Vec<_> = locales
            .iter()
            .map(|locale| normalize_for_lookup(locale.id()))
            .collect();
        locales
            .iter()
            .map(|locale| {
                let mut bundle = Self::default();
                // The first locale that serves a locale is the locale itself.
                for index in lookup_indices(locale, &normalized) {
                    for (id, definition) in &own[index].messages {
                        bundle.messages.entry(id).or_insert(*definition);
                    }
                    for (id, definition) in &own[index].terms {
                        bundle.terms.entry(id).or_insert(*definition);
                    }
                }
                bundle
            })
            .collect()
    }

    /// Returns the messages and terms that `files` define. The first
    /// definition of an ID wins.
    fn own(files: &'s [ParsedFile<'s>]) -> Self {
        let mut bundle = Self::default();
        for entry in files.iter().flat_map(|file| &file.resource.body) {
            match entry {
                Entry::Message(message) => {
                    bundle
                        .messages
                        .entry(message.id.name)
                        .or_insert(Definition {
                            value: message.value.as_ref(),
                            attributes: &message.attributes,
                        });
                }
                Entry::Term(term) => {
                    bundle.terms.entry(term.id.name).or_insert(Definition {
                        value: Some(&term.value),
                        attributes: &term.attributes,
                    });
                }
                _ => {}
            }
        }
        bundle
    }

    /// Returns the pattern of the message `id`, or of its `attribute`, or
    /// `None` when the bundle lacks it.
    fn message(&self, id: &str, attribute: Option<&str>) -> Option<&'s Pattern<&'s str>> {
        self.messages.get(id)?.pattern(attribute)
    }

    /// Adds the variables of `pattern` to `variables`, including the ones of
    /// the messages it references, since those are formatted with the same
    /// arguments.
    fn variables(&self, pattern: &'s Pattern<&'s str>, variables: &mut BTreeSet<&'s str>) {
        let mut followed = HashSet::new();
        let mut pending = vec![pattern];
        while let Some(pattern) = pending.pop() {
            visit_expressions(pattern, &mut |expression| match expression {
                InlineExpression::VariableReference { id } => {
                    variables.insert(id.name);
                }
                InlineExpression::MessageReference { id, attribute } => {
                    let attribute = attribute.as_ref().map(|attribute| attribute.name);
                    if followed.insert((id.name, attribute))
                        && let Some(pattern) = self.message(id.name, attribute)
                    {
                        pending.push(pattern);
                    }
                }
                _ => {}
            });
        }
    }
}

impl<'s> Definition<'s> {
    /// Returns the value, or the pattern of `attribute`.
    fn pattern(&self, attribute: Option<&str>) -> Option<&'s Pattern<&'s str>> {
        match attribute {
            Some(attribute) => self
                .attributes
                .iter()
                .find(|candidate| candidate.id.name == attribute)
                .map(|attribute| &attribute.value),
            None => self.value,
        }
    }
}

/// Reports the keys that locales lack, the variables that calls do not pass,
/// and the arguments that no locale uses.
fn check_calls(
    calls: &[RawTranslation<'_>],
    locales: &[&Locale],
    bundles: &[Bundle<'_>],
    problems: &mut Vec<Problem>,
) {
    for call in calls {
        let key = key(call.id, call.attribute);
        let location = Location {
            file: PathBuf::from(call.file),
            line: call.line,
            column: Some(call.column),
        };
        let mut report = |severity, message| {
            problems.push(Problem {
                severity,
                location: location.clone(),
                message,
            });
        };

        let mut missing = Vec::new();
        let mut variables = BTreeSet::new();
        for (locale, bundle) in locales.iter().zip(bundles) {
            match bundle.message(call.id, call.attribute) {
                Some(pattern) => bundle.variables(pattern, &mut variables),
                None => missing.push(locale.to_string()),
            }
        }
        if missing.len() == locales.len() {
            let message = match suggestion(&key, bundles) {
                Some(suggestion) => {
                    format!("no locale has `{key}`; did you mean `{suggestion}`?")
                }
                None => format!("no locale has `{key}`"),
            };
            report(Severity::Error, message);
            continue;
        }
        if !missing.is_empty() {
            report(
                Severity::Error,
                format!("`{key}` is missing in {}", missing.join(", ")),
            );
        }
        for variable in &variables {
            if !call.arguments.contains(variable) {
                report(
                    Severity::Error,
                    format!("`{key}` uses `${variable}`, but `t!` does not pass it"),
                );
            }
        }
        for argument in &call.arguments {
            if !variables.contains(argument) {
                report(
                    Severity::Warning,
                    format!("no locale uses the argument `{argument}` of `{key}`"),
                );
            }
        }
    }
}

/// Reports the references to messages, terms and attributes that the locale of
/// the referencing file lacks.
fn check_references(
    files: &[Vec<ParsedFile<'_>>],
    bundles: &[Bundle<'_>],
    problems: &mut Vec<Problem>,
) {
    for (files, bundle) in files.iter().zip(bundles) {
        for file in files {
            for pattern in patterns(file) {
                visit_expressions(pattern, &mut |expression| {
                    let (definitions, prefix, id, attribute) = match expression {
                        InlineExpression::MessageReference { id, attribute } => {
                            (&bundle.messages, "", id, attribute)
                        }
                        InlineExpression::TermReference { id, attribute, .. } => {
                            (&bundle.terms, "-", id, attribute)
                        }
                        _ => return,
                    };
                    let attribute = attribute.as_ref().map(|attribute| attribute.name);
                    let message = match definitions.get(id.name) {
                        None => format!("`{prefix}{}` is not defined", id.name),
                        Some(definition) if definition.pattern(attribute).is_none() => {
                            match attribute {
                                Some(attribute) => {
                                    format!("`{prefix}{}` has no attribute `{attribute}`", id.name)
                                }
                                None => format!("`{prefix}{}` has no value", id.name),
                            }
                        }
                        Some(_) => return,
                    };
                    problems.push(Problem {
                        severity: Severity::Error,
                        location: file.location(id.name),
                        message,
                    });
                });
            }
        }
    }
}

/// Reports the messages that no `t!` call and no other message uses, once per
/// message ID.
fn check_unused(
    calls: &[RawTranslation<'_>],
    files: &[Vec<ParsedFile<'_>>],
    problems: &mut Vec<Problem>,
) {
    let mut used: HashSet<&str> = calls.iter().map(|call| call.id).collect();
    for file in files.iter().flatten() {
        for pattern in patterns(file) {
            visit_expressions(pattern, &mut |expression| {
                if let InlineExpression::MessageReference { id, .. } = expression {
                    used.insert(id.name);
                }
            });
        }
    }
    for file in files.iter().flatten() {
        for entry in &file.resource.body {
            if let Entry::Message(message) = entry
                && used.insert(message.id.name)
            {
                problems.push(Problem {
                    severity: Severity::Warning,
                    location: file.location(message.id.name),
                    message: format!("no `t!` uses `{}`", message.id.name),
                });
            }
        }
    }
}

/// Returns the coverage of each locale: which of the keys that `calls` use it
/// lacks.
fn coverage(
    calls: &[RawTranslation<'_>],
    locales: &[&Locale],
    bundles: &[Bundle<'_>],
) -> Vec<Coverage> {
    let keys: BTreeMap<String, &RawTranslation<'_>> = calls
        .iter()
        .map(|call| (key(call.id, call.attribute), call))
        .collect();
    locales
        .iter()
        .zip(bundles)
        .map(|(locale, bundle)| Coverage {
            locale: (*locale).clone(),
            keys: keys.len(),
            missing: keys
                .iter()
                .filter(|(_, call)| bundle.message(call.id, call.attribute).is_none())
                .map(|(key, _)| key.clone())
                .collect(),
        })
        .collect()
}

/// Returns every pattern of the messages and terms of `file`.
fn patterns<'s>(file: &'s ParsedFile<'s>) -> impl Iterator<Item = &'s Pattern<&'s str>> {
    file.resource.body.iter().flat_map(|entry| {
        let (value, attributes) = match entry {
            Entry::Message(message) => (message.value.as_ref(), &message.attributes[..]),
            Entry::Term(term) => (Some(&term.value), &term.attributes[..]),
            _ => (None, &[][..]),
        };
        value
            .into_iter()
            .chain(attributes.iter().map(|attribute| &attribute.value))
    })
}

/// Returns the key of a message, such as `title` or `search.placeholder`.
fn key(id: &str, attribute: Option<&str>) -> String {
    match attribute {
        Some(attribute) => format!("{id}.{attribute}"),
        None => id.to_owned(),
    }
}

/// Returns the key of some locale that is closest to `key`, if one is close
/// enough to be a likely typo.
fn suggestion(key: &str, bundles: &[Bundle<'_>]) -> Option<String> {
    let max_distance = (key.chars().count() / 3).max(1);
    bundles
        .iter()
        .flat_map(|bundle| &bundle.messages)
        .flat_map(|(id, definition)| {
            let value = definition.value.map(|_| (*id).to_owned());
            let attributes = definition
                .attributes
                .iter()
                .map(|attribute| self::key(id, Some(attribute.id.name)));
            value.into_iter().chain(attributes)
        })
        // Swapping two letters counts as one edit.
        .map(|candidate| (strsim::damerau_levenshtein(key, &candidate), candidate))
        .filter(|(distance, _)| *distance <= max_distance)
        .min()
        .map(|(_, candidate)| candidate)
}
