/// Where the browser loads a Fontsource font file from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Host {
    /// Bundle the file as a Topcoat asset and serve it from your own origin.
    #[cfg(feature = "asset")]
    Asset,
    /// Load the file from the [jsDelivr](https://www.jsdelivr.com/) CDN.
    JsDelivr,
}
