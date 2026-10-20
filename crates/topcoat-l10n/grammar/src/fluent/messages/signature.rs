use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    ptr,
};

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Message, Pattern, PatternElement};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use topcoat_core_grammar::paths::topcoat_l10n;

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

    /// Generates a module for each message, named after its ID, reporting
    /// errors at `span`.
    ///
    /// A message's module holds the `Args` of its value and a module for each
    /// of its attributes, which holds the `Args` of the attribute.
    ///
    /// # Errors
    ///
    /// Fails when a name cannot be used in Rust, or when two names become the
    /// same Rust name.
    pub fn expand_all(signatures: &[Self], span: Span) -> syn::Result<TokenStream> {
        let mut names = Names::new(&["CATALOG"]);
        let modules = signatures
            .iter()
            .map(|signature| {
                let name = names.add(&signature.id, span)?;
                signature.expand(&name, span)
            })
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(quote! { #(#modules)* })
    }

    /// Generates the module of this message, named `name`.
    fn expand(&self, name: &Ident, span: Span) -> syn::Result<TokenStream> {
        let id = &self.id;
        let subject = format!("the `{id}` message");
        let doc = format!("The `{id}` message.");
        let value = self
            .value
            .as_ref()
            .map(|variables| Self::args(&subject, variables, span))
            .transpose()?;

        let mut names = Names::new(&["Args", "format"]);
        let attributes = self
            .attributes
            .iter()
            .map(|(attribute, variables)| {
                let name = names.add(attribute, span)?;
                let subject = format!("the `{attribute}` attribute of the `{id}` message");
                let doc = format!("The `{attribute}` attribute of the `{id}` message.");
                let args = Self::args(&subject, variables, span)?;
                Ok(quote! {
                    #[doc = #doc]
                    pub mod #name {
                        #args
                    }
                })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        Ok(quote! {
            #[doc = #doc]
            pub mod #name {
                #value
                #(#attributes)*
            }
        })
    }

    /// Generates the `Args` struct of `subject`, a message or attribute, with
    /// a field for each variable, and its conversion to Fluent arguments.
    fn args(subject: &str, variables: &BTreeSet<String>, span: Span) -> syn::Result<TokenStream> {
        let doc = format!("The arguments of {subject}.");
        let mut names = Names::new(&[]);
        let fields = variables
            .iter()
            .map(|variable| names.add(variable, span))
            .collect::<syn::Result<Vec<_>>>()?;
        let variables = variables.iter();
        let count = fields.len();

        if fields.is_empty() {
            return Ok(quote! {
                #[doc = #doc]
                pub struct Args {}

                impl ::core::convert::From<Args> for #topcoat_l10n::FluentArgs<'_> {
                    fn from(_: Args) -> Self {
                        #topcoat_l10n::FluentArgs::new()
                    }
                }
            });
        }
        // Interpolated in a repetition, which cannot use the `topcoat_l10n`
        // constant directly.
        let value = quote! { #topcoat_l10n::FluentValue<'a> };
        Ok(quote! {
            #[doc = #doc]
            pub struct Args<'a> {
                #(pub #fields: #value,)*
            }

            impl<'a> ::core::convert::From<Args<'a>> for #topcoat_l10n::FluentArgs<'a> {
                fn from(args: Args<'a>) -> Self {
                    let mut fluent = #topcoat_l10n::FluentArgs::with_capacity(#count);
                    #(fluent.set(#variables, args.#fields);)*
                    fluent
                }
            }
        })
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

/// The Rust names given to Fluent names within one scope, such as the
/// messages of a catalog or the variables of a message.
struct Names<'a> {
    /// The Fluent name behind each Rust name given so far.
    given: HashMap<String, &'a str>,
    /// Rust names that generated code already uses in this scope.
    reserved: &'static [&'static str],
}

impl<'a> Names<'a> {
    fn new(reserved: &'static [&'static str]) -> Self {
        Self {
            given: HashMap::new(),
            reserved,
        }
    }

    /// Returns the Rust name of the Fluent name `name`, reporting errors at
    /// `span`.
    ///
    /// Hyphens become underscores, and keywords become raw identifiers, such
    /// as `r#type`.
    fn add(&mut self, name: &'a str, span: Span) -> syn::Result<Ident> {
        let rust = name.replace('-', "_");
        if let Some(other) = self.given.insert(rust.clone(), name) {
            return Err(syn::Error::new(
                span,
                format!("`{other}` and `{name}` both become `{rust}` in Rust"),
            ));
        }
        if self.reserved.contains(&rust.as_str()) {
            return Err(syn::Error::new(
                span,
                format!("`{name}` cannot be used as a name here, because it is reserved"),
            ));
        }
        if let Ok(ident) = syn::parse_str::<Ident>(&rust) {
            return Ok(ident);
        }
        // `syn` rejects keywords as identifiers. These cannot be raw.
        if matches!(rust.as_str(), "self" | "Self" | "super" | "crate") {
            return Err(syn::Error::new(
                span,
                format!("`{name}` cannot be used as a name in Rust"),
            ));
        }
        Ok(Ident::new_raw(&rust, Span::call_site()))
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

    fn names(name: &str) -> syn::Result<String> {
        Names::new(&[])
            .add(name, Span::call_site())
            .map(|ident| ident.to_string())
    }

    #[test]
    fn names_replace_hyphens_with_underscores() {
        assert_eq!(names("login-input").unwrap(), "login_input");
    }

    #[test]
    fn names_make_keywords_raw() {
        assert_eq!(names("type").unwrap(), "r#type");
    }

    #[test]
    fn names_reject_keywords_that_cannot_be_raw() {
        assert!(names("self").is_err());
        assert!(names("crate").is_err());
    }

    #[test]
    fn names_reject_clashes() {
        let mut names = Names::new(&[]);
        assert!(names.add("a-b", Span::call_site()).is_ok());
        assert!(names.add("a_b", Span::call_site()).is_err());
    }

    #[test]
    fn names_reject_reserved_names() {
        let mut names = Names::new(&["Args"]);
        assert!(names.add("Args", Span::call_site()).is_err());
    }

    fn expand(sources: &[&str]) -> syn::Result<syn::File> {
        let signatures = collect(sources);
        let tokens = MessageSignature::expand_all(&signatures, Span::call_site())?;
        Ok(syn::parse2(tokens).expect("the generated code is valid Rust"))
    }

    /// Returns the items of the module `name` among `items`.
    fn module<'a>(items: &'a [syn::Item], name: &str) -> &'a [syn::Item] {
        items
            .iter()
            .find_map(|item| match item {
                syn::Item::Mod(module) if module.ident == name => {
                    Some(&module.content.as_ref().unwrap().1[..])
                }
                _ => None,
            })
            .unwrap()
    }

    /// Returns the field names of the `Args` struct among `items`.
    fn args_fields(items: &[syn::Item]) -> Vec<String> {
        items
            .iter()
            .find_map(|item| match item {
                syn::Item::Struct(args) if args.ident == "Args" => Some(
                    args.fields
                        .iter()
                        .map(|field| field.ident.as_ref().unwrap().to_string())
                        .collect(),
                ),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn generates_a_module_with_args_for_each_message_and_attribute() {
        let file = expand(&[
            "login-input = Log in as { $user-name }\n    .placeholder = Name\nitems = { $count } { $type }\n",
        ])
        .unwrap();
        let login_input = module(&file.items, "login_input");
        assert_eq!(args_fields(login_input), ["user_name"]);
        assert!(args_fields(module(login_input, "placeholder")).is_empty());
        assert_eq!(args_fields(module(&file.items, "items")), ["count", "r#type"]);
    }

    #[test]
    fn generates_no_args_for_a_message_without_a_value() {
        let file = expand(&["input =\n    .placeholder = Name\n"]).unwrap();
        let input = module(&file.items, "input");
        assert!(
            !input
                .iter()
                .any(|item| matches!(item, syn::Item::Struct(_)))
        );
    }

    #[test]
    fn rejects_names_that_clash_with_generated_items() {
        assert!(expand(&["CATALOG = Catalog\n"]).is_err());
        assert!(expand(&["input = Input\n    .Args = Arguments\n"]).is_err());
    }

    #[test]
    fn excludes_the_variables_of_terms() {
        let signatures =
            collect(&["-brand = { $case }\nabout = About { -brand(case: \"genitive\") }\n"]);
        assert_eq!(signatures.len(), 1);
        assert_eq!(signatures[0].value, Some(set(&[])));
    }
}
