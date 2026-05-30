pub type Error = anyhow::Error;
pub type Result<T = (), E = Error> = ::core::result::Result<T, E>;

pub use anyhow::anyhow as error;
pub use anyhow::bail;
pub use anyhow::ensure;
