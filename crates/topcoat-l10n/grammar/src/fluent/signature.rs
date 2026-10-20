use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ptr,
};

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Message, Pattern, PatternElement};

use crate::fluent::LocaleResources;

/// A message as code sees it: its ID, and the variables of its value and of
/// each of its attributes, across all locales.
#[derive(Debug, PartialEq, Eq)]
pub struct MessageSignature {
    /// The ID of the message, as written in Fluent.
    pub id: String,
    /// The variables of the message's value, or `None` when no locale gives
    /// the message a value.
    pub value: Option<BTreeSet<String>>,
    /// The variables of each attribute, by attribute name.
    pub attributes: BTreeMap<String, BTreeSet<String>>,
}

impl MessageSignature {
    /// Returns the signatures of the messages that any of `locales` defines,
    /// sorted by ID.
    ///
    /// A message's variables are those any locale uses in it, including the
    /// variables of the messages it references, since a reference is formatted
    /// with the same arguments. Variables within terms are not included, since
    /// a term only sees the arguments it is called with.
    #[must_use]
    pub fn collect(locales: &[LocaleResources]) -> Vec<Self> {
        let mut signatures = BTreeMap::<&str, Self>::new();
        for locale in locales {
            let messages = LocaleMessages::new(locale);
            for (&id, message) in &messages.messages {
                let signature = signatures.entry(id).or_insert_with(|| Self {
                    id: id.to_owned(),
                    value: None,
                    attributes: BTreeMap::new(),
                });
                if let Some(value) = &message.value {
                    messages.variables(value, signature.value.get_or_insert_default());
                }
                for attribute in &message.attributes {
                    let variables = signature
                        .attributes
                        .entry(attribute.id.name.clone())
                        .or_default();
                    messages.variables(&attribute.value, variables);
                }
            }
        }
        signatures.into_values().collect()
    }
}

/// The messages of one locale, by ID.
struct LocaleMessages<'a> {
    messages: HashMap<&'a str, &'a Message<String>>,
}

impl<'a> LocaleMessages<'a> {
    fn new(locale: &'a LocaleResources) -> Self {
        let messages = locale
            .files
            .iter()
            .flat_map(|file| &file.resource.body)
            .filter_map(|entry| match entry {
                Entry::Message(message) => Some((message.id.name.as_str(), message)),
                _ => None,
            })
            .collect();
        Self { messages }
    }

    /// Adds the variables of `pattern` to `variables`.
    fn variables(&self, pattern: &'a Pattern<String>, variables: &mut BTreeSet<String>) {
        let mut walk = Walk {
            messages: self,
            variables,
            visited: HashSet::new(),
        };
        walk.pattern(pattern);
    }

    /// Returns the pattern of a message's value or of one of its attributes.
    fn pattern(&self, id: &str, attribute: Option<&str>) -> Option<&'a Pattern<String>> {
        let message = self.messages.get(id)?;
        match attribute {
            Some(attribute) => message
                .attributes
                .iter()
                .find(|candidate| candidate.id.name == attribute)
                .map(|attribute| &attribute.value),
            None => message.value.as_ref(),
        }
    }
}

/// Collects the variables of patterns, following message references.
struct Walk<'m, 'a, 'v> {
    messages: &'m LocaleMessages<'a>,
    variables: &'v mut BTreeSet<String>,
    /// The patterns already walked, which stops cyclic references.
    visited: HashSet<*const Pattern<String>>,
}

impl<'a> Walk<'_, 'a, '_> {
    fn pattern(&mut self, pattern: &'a Pattern<String>) {
        if !self.visited.insert(ptr::from_ref(pattern)) {
            return;
        }
        for element in &pattern.elements {
            if let PatternElement::Placeable { expression } = element {
                self.expression(expression);
            }
        }
    }

    fn expression(&mut self, expression: &'a Expression<String>) {
        match expression {
            Expression::Select { selector, variants } => {
                self.inline(selector);
                for variant in variants {
                    self.pattern(&variant.value);
                }
            }
            Expression::Inline(expression) => self.inline(expression),
        }
    }

    fn inline(&mut self, expression: &'a InlineExpression<String>) {
        match expression {
            InlineExpression::VariableReference { id } => {
                self.variables.insert(id.name.clone());
            }
            InlineExpression::MessageReference { id, attribute } => {
                let attribute = attribute.as_ref().map(|attribute| attribute.name.as_str());
                if let Some(pattern) = self.messages.pattern(&id.name, attribute) {
                    self.pattern(pattern);
                }
            }
            // Named arguments are always literals.
            InlineExpression::FunctionReference { arguments, .. } => {
                for argument in &arguments.positional {
                    self.inline(argument);
                }
            }
            InlineExpression::Placeable { expression } => self.expression(expression),
            // A term only takes named arguments, which are always literals.
            InlineExpression::TermReference { .. }
            | InlineExpression::StringLiteral { .. }
            | InlineExpression::NumberLiteral { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fluent_syntax::parser;
    use icu_locale_core::LanguageIdentifier;

    use super::*;
    use crate::fluent::FluentFile;

    fn locale(source: &str) -> LocaleResources {
        LocaleResources {
            locale: LanguageIdentifier::UNKNOWN,
            files: vec![FluentFile {
                path: PathBuf::new(),
                resource: parser::parse(source.to_owned()).unwrap(),
            }],
        }
    }

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn collect(sources: &[&str]) -> Vec<MessageSignature> {
        let locales: Vec<_> = sources.iter().map(|source| locale(source)).collect();
        MessageSignature::collect(&locales)
    }

    fn find<'a>(signatures: &'a [MessageSignature], id: &str) -> &'a MessageSignature {
        signatures
            .iter()
            .find(|signature| signature.id == id)
            .unwrap()
    }

    #[test]
    fn collects_messages_sorted_by_id() {
        let signatures = collect(&["b = B\na = A { $name }\n"]);
        let ids: Vec<_> = signatures
            .iter()
            .map(|signature| signature.id.as_str())
            .collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(signatures[0].value, Some(set(&["name"])));
        assert_eq!(signatures[1].value, Some(set(&[])));
    }

    #[test]
    fn collects_attributes_separately() {
        let signatures = collect(&["input = Log in\n    .placeholder = Hi { $name }\n"]);
        assert_eq!(signatures[0].value, Some(set(&[])));
        assert_eq!(
            signatures[0].attributes,
            BTreeMap::from([("placeholder".to_owned(), set(&["name"]))])
        );
    }

    #[test]
    fn has_no_value_when_no_locale_gives_one() {
        let signatures = collect(&["input =\n    .placeholder = Name\n"]);
        assert_eq!(signatures[0].value, None);
    }

    #[test]
    fn combines_the_messages_and_variables_of_all_locales() {
        let signatures = collect(&[
            "hi = Hi { $name }\n",
            "hi = Hallo { $title } { $name }\nbye = Tschuess\n",
        ]);
        assert_eq!(find(&signatures, "hi").value, Some(set(&["name", "title"])));
        assert_eq!(find(&signatures, "bye").value, Some(set(&[])));
    }

    #[test]
    fn finds_variables_in_selectors_variants_and_function_calls() {
        let signatures = collect(&[
            "items = { $count ->\n    [one] One { $kind }\n   *[other] { NUMBER($amount, minimumFractionDigits: 2) }\n}\n",
        ]);
        assert_eq!(
            signatures[0].value,
            Some(set(&["amount", "count", "kind"]))
        );
    }

    #[test]
    fn includes_the_variables_of_referenced_messages() {
        let signatures = collect(&[
            "greeting = Hi { $name }\nwelcome = { greeting }, { login.label }\nlogin = Log in\n    .label = as { $user }\n",
        ]);
        assert_eq!(
            find(&signatures, "welcome").value,
            Some(set(&["name", "user"]))
        );
    }

    #[test]
    fn stops_at_cyclic_references() {
        let signatures = collect(&["a = { b } { $x }\nb = { a } { $y }\n"]);
        assert_eq!(find(&signatures, "a").value, Some(set(&["x", "y"])));
        assert_eq!(find(&signatures, "b").value, Some(set(&["x", "y"])));
    }

    #[test]
    fn excludes_the_variables_of_terms() {
        let signatures =
            collect(&["-brand = { $case }\nabout = About { -brand(case: \"genitive\") }\n"]);
        assert_eq!(signatures.len(), 1);
        assert_eq!(signatures[0].value, Some(set(&[])));
    }
}
