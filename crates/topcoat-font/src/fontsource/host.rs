/// Place a Fontsource font should be loaded from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Host {
    /// Download the Fontsource font and self-host it as a Topcoat asset.
    #[cfg(feature = "asset")]
    Asset,
    /// Load the Fontsource font from <https://www.jsdelivr.com/>.
    JsDelivr,
}
