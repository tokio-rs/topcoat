use std::sync::Arc;

use topcoat_core::base_url::BaseUrl;

/// The browser origins allowed for every WebSocket route in an application.
#[derive(Clone, Debug, Default)]
pub(crate) struct WebSocketOrigins(Option<Arc<[String]>>);

impl WebSocketOrigins {
    /// Replaces the application-wide origin allowlist.
    pub(crate) fn replace<I>(&mut self, origins: I)
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.0 = Some(
            origins
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into(),
        );
    }

    /// Uses `origin` when no allowlist was configured.
    pub(crate) fn default_to(&mut self, origin: Option<String>) {
        self.0
            .get_or_insert_with(|| origin.into_iter().collect::<Vec<_>>().into());
    }

    /// Returns the configured origins.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &str> {
        self.0
            .iter()
            .flat_map(|origins| origins.iter())
            .map(String::as_str)
    }

    /// Returns the origin string of a base URL without its path prefix.
    pub(crate) fn base_url_origin(base_url: &BaseUrl) -> String {
        let uri: http::Uri = base_url
            .as_str()
            .parse()
            .expect("a BaseUrl always contains a valid URI");
        let scheme = uri
            .scheme_str()
            .expect("a BaseUrl always contains a scheme");
        let authority = uri
            .authority()
            .expect("a BaseUrl always contains an authority");
        let port = authority
            .port_u16()
            .filter(|port| !matches!((scheme, port), ("http", 80) | ("https", 443)));
        match port {
            Some(port) => format!("{scheme}://{}:{port}", authority.host()),
            None => format!("{scheme}://{}", authority.host()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_the_base_url_origin() {
        let base_url = BaseUrl::new("https://app.example:443/prefix").unwrap();
        let mut origins = WebSocketOrigins::default();

        origins.default_to(Some(WebSocketOrigins::base_url_origin(&base_url)));

        assert_eq!(origins.iter().collect::<Vec<_>>(), ["https://app.example"]);
    }

    #[test]
    fn configured_origins_override_the_base_url() {
        let base_url = BaseUrl::new("https://app.example").unwrap();
        let mut origins = WebSocketOrigins::default();
        origins.replace(["https://admin.example"]);

        origins.default_to(Some(WebSocketOrigins::base_url_origin(&base_url)));

        assert_eq!(
            origins.iter().collect::<Vec<_>>(),
            ["https://admin.example"]
        );
    }

    #[test]
    fn an_empty_configuration_overrides_the_base_url() {
        let base_url = BaseUrl::new("https://app.example").unwrap();
        let mut origins = WebSocketOrigins::default();
        origins.replace([] as [&str; 0]);

        origins.default_to(Some(WebSocketOrigins::base_url_origin(&base_url)));

        assert_eq!(origins.iter().count(), 0);
    }
}
