/// Why a page cannot be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageError {
    /// The page does not exist, rendered as the 404 page.
    NotFound,
    /// The request is malformed, rendered as the 422 page.
    Invalid,
}
