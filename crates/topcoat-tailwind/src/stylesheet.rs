/// Returns the generated Tailwind stylesheet as a Topcoat asset.
///
/// Uses the default build output at `$OUT_DIR/tailwind.css`. For a custom
/// output path, declare the stylesheet with `asset!` directly.
///
/// ```rust,ignore
/// view! {
///     <link rel="stylesheet" href=(tailwind::stylesheet!())>
/// }
/// ```
#[macro_export]
macro_rules! stylesheet {
    () => {
        ::topcoat::asset::asset!(concat!(env!("OUT_DIR"), "/tailwind.css"))
    };
}
