use std::collections::HashMap;

use syn::parse::Parse;

use crate::pretty::{Lexer, Macro, MacroSnippet, PrettyPrint, Printer};

type MacroPrettyPrintFn = fn(&Registry, &MacroSnippet) -> syn::Result<String>;

#[derive(Default)]
pub struct Registry {
    macro_fns: HashMap<String, MacroPrettyPrintFn>,
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
        result.register_macro::<T>(name);
        result
    }

    /// Registers a pretty-printer for the macro named `name`.
    ///
    /// # Panics
    ///
    /// Panics if a pretty-printer has already been registered under `name`.
    pub fn register_macro<T>(&mut self, name: impl Into<String>) -> &mut Self
    where
        T: Parse + PrettyPrint,
    {
        let name = name.into();
        let pretty_print_fn = |registry: &Registry, snippet: &MacroSnippet| {
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
        };

        assert!(
            !self.macro_fns.contains_key(&name),
            "registered multiple pretty print macros under the name `{name}`",
        );

        self.macro_fns.insert(name, pretty_print_fn);

        self
    }

    #[must_use]
    pub fn pretty_print_macro(&self, snippet: &MacroSnippet) -> Option<syn::Result<String>> {
        self.macro_fns
            .get(snippet.name())
            .map(|pretty_print_fn| pretty_print_fn(self, snippet))
    }
}
