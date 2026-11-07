//! Email addresses with an optional display name.

use core::fmt;
use std::str::FromStr;

/// A validated email address with an optional display name.
///
/// Renders as `Ada Lovelace <ada@example.com>` or `ada@example.com`.
/// Addresses are validated at construction.
///
/// Construct one with [`Mailbox::new`] or [`Mailbox::named`], or parse the
/// bare or display-name form:
///
/// ```
/// use topcoat_mail::Mailbox;
///
/// let bare = Mailbox::new("ada@example.com")?;
/// let named = Mailbox::named("Ada Lovelace", "ada@example.com")?;
/// let parsed: Mailbox = "Ada Lovelace <ada@example.com>".parse()?;
/// # Ok::<(), topcoat_mail::AddressError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mailbox {
    name: Option<String>,
    address: lettre::Address,
}

impl Mailbox {
    /// A bare address with no display name.
    ///
    /// The string must be a plain `local@domain` address; use
    /// [`str::parse`] for the `Ada Lovelace <ada@example.com>` form.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError`] if the string is not a valid address.
    pub fn new(address: impl AsRef<str>) -> Result<Mailbox, AddressError> {
        Ok(Mailbox {
            name: None,
            address: address.as_ref().parse().map_err(AddressError)?,
        })
    }

    /// Creates an address with a display name, such as `Ada <ada@example.com>`.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError`] if the address is not a valid address.
    pub fn named(
        name: impl Into<String>,
        address: impl AsRef<str>,
    ) -> Result<Mailbox, AddressError> {
        Ok(Mailbox {
            name: Some(name.into()),
            address: address.as_ref().parse().map_err(AddressError)?,
        })
    }

    /// The display name, if any.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The address itself (`local@domain`).
    #[must_use]
    pub fn address(&self) -> &str {
        self.address.as_ref()
    }

    /// Converts into the lettre mailbox used during MIME assembly.
    pub(crate) fn to_lettre(&self) -> lettre::message::Mailbox {
        lettre::message::Mailbox::new(self.name.clone(), self.address.clone())
    }
}

impl fmt::Display for Mailbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{name} <{}>", self.address),
            None => write!(f, "{}", self.address),
        }
    }
}

/// Parses the bare (`ada@example.com`) or display-name
/// (`Ada Lovelace <ada@example.com>`) form.
impl FromStr for Mailbox {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Mailbox, AddressError> {
        let mailbox = s
            .parse::<lettre::message::Mailbox>()
            .map_err(AddressError)?;
        Ok(Mailbox {
            name: mailbox.name,
            address: mailbox.email,
        })
    }
}

/// The reason a string was rejected as an email address.
#[derive(Clone, Debug, thiserror::Error)]
#[error("invalid email address: {0}")]
pub struct AddressError(lettre::address::AddressError);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_addresses_from_every_constructor() -> Result<(), AddressError> {
        let bare = Mailbox::new("ada@example.com")?;
        assert_eq!(bare.name(), None);
        assert_eq!(bare.address(), "ada@example.com");

        let named = Mailbox::named("Ada Lovelace", "ada@example.com")?;
        assert_eq!(named.name(), Some("Ada Lovelace"));
        assert_eq!(named.address(), "ada@example.com");

        let parsed: Mailbox = "Ada Lovelace <ada@example.com>".parse()?;
        assert_eq!(parsed, named);

        Ok(())
    }

    #[test]
    fn rejects_invalid_addresses() {
        assert!(Mailbox::new("not-an-address").is_err());
        assert!(Mailbox::new("Ada <ada@example.com>").is_err());
        assert!("no-at-sign".parse::<Mailbox>().is_err());
        assert!("@example.com".parse::<Mailbox>().is_err());
        assert!(Mailbox::named("Ada", "nope").is_err());
    }

    #[test]
    fn displays_the_mailbox_form() -> Result<(), AddressError> {
        let bare = Mailbox::new("ada@example.com")?;
        assert_eq!(bare.to_string(), "ada@example.com");

        let named = Mailbox::named("Ada Lovelace", "ada@example.com")?;
        assert_eq!(named.to_string(), "Ada Lovelace <ada@example.com>");

        Ok(())
    }
}
