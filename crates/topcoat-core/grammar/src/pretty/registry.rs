use std::collections::HashMap;

use syn::parse::Parse;

use crate::pretty::{Lexer, Macro, MacroKind, MacroSnippet, PrettyPrint, Printer};

type MacroPrettyPrintFn = fn(&Registry, &MacroSnippet) -> syn::Result<String>;

#[derive(Default)]
pub struct Registry {
    function_like_fns: HashMap<String, MacroPrettyPrintFn>,
    attribute_fns: HashMap<String, MacroPrettyPrintFn>,
}

impl Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn one<T>(name: impl Into<String>) -> Self
    where
        T: Parse + PrettyPrint,
    {
        let mut result = Self::default();
        result.register_function_like::<T>(name);
        result
    }

    /// Registers a pretty-printer for the function-like macro named `name`.
    ///
    /// # Panics
    ///
    /// Panics if a pretty-printer has already been registered for a function-like
    /// macro under `name`.
    pub fn register_function_like<T>(&mut self, name: impl Into<String>) -> &mut Self
    where
        T: Parse + PrettyPrint,
    {
        let name = name.into();
        assert!(
            !self.function_like_fns.contains_key(&name),
            "registered multiple pretty print function-like macros under the name `{name}`",
        );
        self.function_like_fns.insert(name, pretty_print::<T>);
        self
    }

    /// Registers a pretty-printer for the arguments of the attribute named
    /// `name`.
    ///
    /// Attributes are named separately from function-like macros, so
    /// `#[name(...)]` and `name!(...)` can have different pretty-printers.
    ///
    /// # Panics
    ///
    /// Panics if a pretty-printer has already been registered for an
    /// attribute under `name`.
    pub fn register_attribute<T>(&mut self, name: impl Into<String>) -> &mut Self
    where
        T: Parse + PrettyPrint,
    {
        let name = name.into();
        assert!(
            !self.attribute_fns.contains_key(&name),
            "registered multiple pretty print attributes under the name `{name}`",
        );
        self.attribute_fns.insert(name, pretty_print::<T>);
        self
    }

    /// Formats `snippet` with the pretty-printer registered for its name and
    /// kind. Returns `None` when no pretty-printer is registered for it.
    #[must_use]
    pub fn pretty_print_macro(&self, snippet: &MacroSnippet) -> Option<syn::Result<String>> {
        let fns = match snippet.kind() {
            MacroKind::FunctionLike => &self.function_like_fns,
            MacroKind::Attribute => &self.attribute_fns,
        };
        fns.get(snippet.name())
            .map(|pretty_print_fn| pretty_print_fn(self, snippet))
    }
}

fn pretty_print<T>(registry: &Registry, snippet: &MacroSnippet) -> syn::Result<String>
where
    T: Parse + PrettyPrint,
{
    let ast: Macro<T> = syn::parse_str(snippet.source_text())?;
    let trivia = Lexer::new(snippet.source_text()).collect::<Vec<_>>();
    let mut printer = Printer::new(
        registry,
        &trivia,
        snippet.initial_space(),
        snippet.initial_indent(),
    );
    ast.pretty_print(&mut printer);
    Ok(printer.eof())
}
