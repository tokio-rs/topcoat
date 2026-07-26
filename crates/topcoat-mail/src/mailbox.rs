//! Email addresses with an optional display name.

use core::fmt;
use std::str::FromStr;

/// A validated email address with an optional display name -- renders as
/// `Ada Lovelace <ada@example.com>` or bare `ada@example.com`.
///
/// The pairing of an address with the human-readable name mail clients
/// show next to it is what RFC 5322 calls a "mailbox". The address is
/// parsed at construction, so every value of this type holds a well-formed
/// address.
///
/// Construct one with [`Mailbox::new`] or [`Mailbox::named`], parse the
/// bare or display-name form, or convert a `(name, address)` pair with
/// `TryInto`:
///
/// ```
/// use topcoat_mail::Mailbox;
///
/// let bare = Mailbox::new("ada@example.com")?;
/// let named: Mailbox = "Ada Lovelace <ada@example.com>".parse()?;
/// let paired: Mailbox = ("Ada Lovelace", "ada@example.com").try_into()?;
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

    /// An address with a display name -- `Ada Lovelace <ada@example.com>`.
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

impl TryFrom<&str> for Mailbox {
    type Error = AddressError;

    fn try_from(address: &str) -> Result<Mailbox, AddressError> {
        address.parse()
    }
}

impl TryFrom<String> for Mailbox {
    type Error = AddressError;

    fn try_from(address: String) -> Result<Mailbox, AddressError> {
        address.parse()
    }
}

impl TryFrom<&String> for Mailbox {
    type Error = AddressError;

    fn try_from(address: &String) -> Result<Mailbox, AddressError> {
        address.parse()
    }
}

/// Any `(name, address)` pair converts, whatever the string flavors --
/// `("Ada", "ada@example.com")`, `(&user.name, &user.email)`, owned
/// `String`s, or a mix.
impl<N, A> TryFrom<(N, A)> for Mailbox
where
    N: Into<String>,
    A: AsRef<str>,
{
    type Error = AddressError;

    fn try_from((name, address): (N, A)) -> Result<Mailbox, AddressError> {
        Mailbox::named(name, address)
    }
}

impl From<&Mailbox> for Mailbox {
    fn from(mailbox: &Mailbox) -> Mailbox {
        mailbox.clone()
    }
}

/// One or more mailboxes, converted fallibly from a single value or a
/// collection.
///
/// The `mail!` macro's recipient fields accept anything implementing this
/// trait: a single [`Mailbox`], address string, or `(name, address)` pair,
/// or a `Vec`, array, or slice of such values. Collection elements convert
/// through `TryInto<Mailbox>`, so the flavors compose:
///
/// ```
/// use topcoat_mail::{Mailbox, TryIntoMailboxes};
///
/// let one = "ada@example.com".try_into_mailboxes()?;
/// let many = [("Ada", "ada@example.com"), ("Bob", "bob@example.com")].try_into_mailboxes()?;
///
/// assert_eq!(one.len(), 1);
/// assert_eq!(many.len(), 2);
/// # Ok::<(), topcoat_mail::AddressError>(())
/// ```
pub trait TryIntoMailboxes {
    /// Converts into mailboxes, parsing addresses as needed.
    ///
    /// # Errors
    ///
    /// Returns [`AddressError`] if any value is not a valid address.
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError>;
}

impl TryIntoMailboxes for Mailbox {
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        Ok(vec![self])
    }
}

impl TryIntoMailboxes for &Mailbox {
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        Ok(vec![self.clone()])
    }
}

impl TryIntoMailboxes for &str {
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        Ok(vec![self.parse()?])
    }
}

impl TryIntoMailboxes for String {
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        Ok(vec![self.parse()?])
    }
}

impl TryIntoMailboxes for &String {
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        Ok(vec![self.parse()?])
    }
}

impl<N, A> TryIntoMailboxes for (N, A)
where
    N: Into<String>,
    A: AsRef<str>,
{
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        Ok(vec![self.try_into()?])
    }
}

impl<T> TryIntoMailboxes for Vec<T>
where
    T: TryInto<Mailbox>,
    T::Error: Into<AddressError>,
{
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        self.into_iter()
            .map(|value| value.try_into().map_err(Into::into))
            .collect()
    }
}

impl<T, const N: usize> TryIntoMailboxes for [T; N]
where
    T: TryInto<Mailbox>,
    T::Error: Into<AddressError>,
{
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        self.into_iter()
            .map(|value| value.try_into().map_err(Into::into))
            .collect()
    }
}

impl<T> TryIntoMailboxes for &[T]
where
    T: Clone + TryInto<Mailbox>,
    <T as TryInto<Mailbox>>::Error: Into<AddressError>,
{
    fn try_into_mailboxes(self) -> Result<Vec<Mailbox>, AddressError> {
        self.iter()
            .cloned()
            .map(|value| value.try_into().map_err(Into::into))
            .collect()
    }
}

/// The reason a string was rejected as an email address.
#[derive(Clone, Debug, thiserror::Error)]
#[error("invalid email address: {0}")]
pub struct AddressError(lettre::address::AddressError);

/// Lets infallible mailbox conversions (from a [`Mailbox`] itself) satisfy
/// the fallible element bound of the [`TryIntoMailboxes`] collections.
impl From<std::convert::Infallible> for AddressError {
    fn from(infallible: std::convert::Infallible) -> AddressError {
        match infallible {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_addresses_from_every_flavor() -> Result<(), AddressError> {
        let owned_name = "Ada".to_owned();
        let owned_addr = "ada@example.com".to_owned();

        let bare = Mailbox::new("ada@example.com")?;
        assert_eq!(bare.name(), None);
        assert_eq!(bare.address(), "ada@example.com");

        let parsed: Mailbox = "Ada Lovelace <ada@example.com>".parse()?;
        assert_eq!(parsed.name(), Some("Ada Lovelace"));
        assert_eq!(parsed.address(), "ada@example.com");

        let expected = Mailbox::named("Ada", "ada@example.com")?;
        let strs: Mailbox = ("Ada", "ada@example.com").try_into()?;
        let mixed: Mailbox = (&owned_name, "ada@example.com").try_into()?;
        let refs: Mailbox = (&owned_name, &owned_addr).try_into()?;
        assert_eq!(strs, expected);
        assert_eq!(mixed, expected);
        assert_eq!(refs, expected);

        Ok(())
    }

    #[test]
    fn converts_singles_and_collections_into_mailboxes() -> Result<(), AddressError> {
        let ada = Mailbox::new("ada@example.com")?;
        let bob = Mailbox::named("Bob", "bob@example.com")?;
        let one = std::slice::from_ref(&ada);

        assert_eq!(ada.clone().try_into_mailboxes()?, one);
        assert_eq!((&ada).try_into_mailboxes()?, one);
        assert_eq!("ada@example.com".try_into_mailboxes()?, one);
        assert_eq!(
            ("Bob", "bob@example.com").try_into_mailboxes()?,
            std::slice::from_ref(&bob)
        );

        let both = vec![ada.clone(), bob.clone()];
        assert_eq!(both.clone().try_into_mailboxes()?, both);
        assert_eq!(both.as_slice().try_into_mailboxes()?, both);
        assert_eq!(
            ["ada@example.com", "bob@example.com"].try_into_mailboxes()?,
            [ada, Mailbox::new("bob@example.com")?]
        );

        assert!(["ada@example.com", "nope"].try_into_mailboxes().is_err());

        Ok(())
    }

    #[test]
    fn rejects_invalid_addresses() {
        assert!(Mailbox::new("not-an-address").is_err());
        assert!(Mailbox::new("Ada <ada@example.com>").is_err());
        assert!("no-at-sign".parse::<Mailbox>().is_err());
        assert!(Mailbox::try_from("@example.com").is_err());
        assert!(Mailbox::try_from(("Ada", "nope")).is_err());
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
