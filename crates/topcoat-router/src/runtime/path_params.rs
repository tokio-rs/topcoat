use std::sync::Arc;

use percent_encoding::percent_decode_str;

/// The path parameters captured from the matched route.
///
/// Iterating over a reference yields each `(name, value)` pair as string
/// slices, with values percent-decoded. This backs the typed `#[path_param]`
/// accessors and is rarely used directly.
#[derive(Debug, Clone, Default)]
pub struct RawPathParams(Vec<(Arc<str>, Box<str>)>);

impl RawPathParams {
    /// Captures the parameters of a route match, percent-decoding each value.
    pub(crate) fn from_pairs<'pairs>(
        pairs: impl IntoIterator<Item = (&'pairs str, &'pairs str)>,
    ) -> Self {
        Self(
            pairs
                .into_iter()
                .map(|(key, value)| {
                    let value = percent_decode_str(value)
                        .decode_utf8_lossy()
                        .into_owned()
                        .into_boxed_str();
                    (Arc::from(key), value)
                })
                .collect(),
        )
    }
}

impl<'params> IntoIterator for &'params RawPathParams {
    type Item = (&'params str, &'params str);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'params, (Arc<str>, Box<str>)>,
        fn(&'params (Arc<str>, Box<str>)) -> (&'params str, &'params str),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|(key, value)| (&**key, &**value))
    }
}
