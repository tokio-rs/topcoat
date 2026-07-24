use crate::AssetBundle;

/// Where the bundled assets are hosted.
pub(crate) enum Host {
    /// Served by the application itself under the internal asset route prefix.
    #[cfg(feature = "serve")]
    Serve,
    /// Hosted externally; asset URLs are formed against this base URL.
    External { base_url: String },
}

/// Asset configuration, registered on the router (with the router's `assets`
/// extension method).
///
/// Assemble one with [`Config::builder`]; `Config::default()` is the
/// all-defaults configuration, loading the bundle from a conventional
/// location (see [`AssetBundle::load`]) and serving it from the application.
/// An [`AssetBundle`] converts into the configuration hosting that bundle
/// with the defaults, so an explicitly loaded bundle registers directly as
/// `.assets(bundle)`.
pub struct Config {
    pub(crate) bundle: AssetBundle,
    pub(crate) host: Host,
}

impl Config {
    /// Creates a builder for an asset configuration.
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }
}

/// Builds the all-defaults configuration, like [`Config::builder`] with an
/// immediate [`build`](ConfigBuilder::build).
impl Default for Config {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Converts a bundle into the configuration hosting it with the defaults.
impl From<AssetBundle> for Config {
    fn from(bundle: AssetBundle) -> Self {
        Config::builder().bundle(bundle).build()
    }
}

/// Assembles a [`Config`]. Created with [`Config::builder`].
#[derive(Default)]
pub struct ConfigBuilder {
    bundle: Option<AssetBundle>,
    host: Option<Host>,
}

impl ConfigBuilder {
    /// Sets the asset bundle, instead of loading it from a conventional
    /// location (see [`AssetBundle::load`]) when the configuration is built.
    #[must_use]
    pub fn bundle(mut self, bundle: AssetBundle) -> Self {
        self.bundle = Some(bundle);
        self
    }

    /// Hosts the bundled assets externally at `base_url` instead of serving
    /// them from the application.
    ///
    /// No asset routes are registered: the files in the bundle directory must
    /// be made available under `base_url` by other means, such as a CDN or
    /// the reverse proxy in front of the application. Each asset's URL is
    /// `{base_url}/{bundled-filename}`; a trailing `/` on `base_url` is
    /// ignored. Bundled filenames are content-hashed, so the files can be
    /// served with long-lived, immutable caching.
    #[must_use]
    pub fn hosted_at(mut self, base_url: impl Into<String>) -> Self {
        let mut base_url = base_url.into();
        while base_url.ends_with('/') {
            base_url.pop();
        }
        self.host = Some(Host::External { base_url });
        self
    }

    /// Consumes the builder, returning the finished [`Config`].
    ///
    /// # Panics
    ///
    /// Panics when no bundle was set and none is found at a conventional
    /// location (see [`AssetBundle::load`]), or when no external host was
    /// configured and the application cannot serve the assets itself because
    /// the `serve` feature is disabled.
    #[must_use]
    pub fn build(self) -> Config {
        Config {
            bundle: self.bundle.unwrap_or_else(default_bundle),
            host: self.host.unwrap_or_else(default_host),
        }
    }
}

fn default_bundle() -> AssetBundle {
    match AssetBundle::load() {
        Ok(bundle) => bundle,
        Err(error) => panic!(
            "no asset bundle configured, and loading one from a conventional location failed: {error}"
        ),
    }
}

#[cfg(feature = "serve")]
fn default_host() -> Host {
    Host::Serve
}

#[cfg(not(feature = "serve"))]
fn default_host() -> Host {
    panic!(
        "no asset host configured: set one with `ConfigBuilder::hosted_at`, or enable the `serve` feature to serve assets from the application"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_at_trims_trailing_slashes() {
        let config = Config::builder()
            .bundle(AssetBundle::empty())
            .hosted_at("https://cdn.example.com/assets/")
            .build();

        match config.host {
            Host::External { base_url } => assert_eq!(base_url, "https://cdn.example.com/assets"),
            #[cfg(feature = "serve")]
            Host::Serve => panic!("expected an external host"),
        }
    }
}
