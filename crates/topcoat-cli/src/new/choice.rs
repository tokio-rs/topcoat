use clap::ValueEnum;

/// How request paths are assigned to handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Routing {
    /// Paths derived from Rust modules
    Module,
    /// Explicit paths in handler attributes, registered automatically
    Discover,
    /// Explicit paths in handler attributes, registered by hand
    Manual,
}

/// The database integration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Database {
    /// Toasty, an async ORM
    Toasty,
    /// No database integration
    None,
}

/// The database backend used by the database integration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum DatabaseBackend {
    /// A local SQLite database file
    #[default]
    Sqlite,
}

/// The approach used for browser interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Interaction {
    /// Topcoat's reactive runtime
    Topcoat,
    /// htmx
    Htmx,
    /// Datastar
    Datastar,
    /// Alpine with Alpine AJAX
    AlpineAjax,
    /// No browser interaction library
    None,
}

/// The source of icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Icons {
    /// Icon sets from Iconify, checked at compile time
    Iconify,
    /// Hand-written SVG icons
    Custom,
    /// No icon integration
    None,
}

/// The source of web fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Font {
    /// A font family from the Fontsource catalog
    Fontsource,
    /// No font integration
    None,
}

/// Returns the command-line name of `value`, e.g. `toasty`.
pub fn value_name<T: ValueEnum>(value: &T) -> String {
    value
        .to_possible_value()
        .expect("choice enums have no skipped variants")
        .get_name()
        .to_string()
}
