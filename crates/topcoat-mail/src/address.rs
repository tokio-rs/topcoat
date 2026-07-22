//! Email addresses with an optional display name.

/// An email address with an optional display name -- renders as
/// `Ada Lovelace <ada@example.com>` or bare `ada@example.com`.
///
/// This is what RFC 5322 calls a "mailbox": the address itself plus the
/// human-readable name mail clients show next to it.
///
/// Most builder methods (`to`, `cc`, `from`, ...) accept
/// `impl Into<MailAddress>`, so a bare address (`"ada@example.com"`), any
/// `(name, address)` pair whose halves are `Into<String>`
/// (`(&user.name, &user.email)` works directly), or a `&MailAddress` all
/// convert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAddress {
    /// Display name, if any.
    pub name: Option<String>,
    /// The address itself (`local@domain`).
    pub address: String,
}

impl MailAddress {
    /// A bare address with no display name.
    #[must_use]
    pub fn new(address: impl Into<String>) -> MailAddress {
        MailAddress {
            name: None,
            address: address.into(),
        }
    }

    /// An address with a display name -- `Ada Lovelace <ada@example.com>`.
    #[must_use]
    pub fn named(name: impl Into<String>, address: impl Into<String>) -> MailAddress {
        MailAddress {
            name: Some(name.into()),
            address: address.into(),
        }
    }

    /// Parse the address for configuration and message validation.
    #[expect(dead_code, reason = "called once mails are assembled for sending")]
    pub(crate) fn parse_address(&self) -> Result<lettre::Address, String> {
        self.address
            .parse::<lettre::Address>()
            .map_err(|error| error.to_string())
    }
}

impl From<&str> for MailAddress {
    fn from(address: &str) -> MailAddress {
        MailAddress::new(address)
    }
}

impl From<String> for MailAddress {
    fn from(address: String) -> MailAddress {
        MailAddress::new(address)
    }
}

impl From<&String> for MailAddress {
    fn from(address: &String) -> MailAddress {
        MailAddress::new(address.clone())
    }
}

/// Any `(name, address)` pair converts, whatever the string flavors --
/// `("Ada", "ada@example.com")`, `(&user.name, &user.email)`, owned
/// `String`s, or a mix.
impl<N, A> From<(N, A)> for MailAddress
where
    N: Into<String>,
    A: Into<String>,
{
    fn from((name, address): (N, A)) -> MailAddress {
        MailAddress::named(name, address)
    }
}

impl From<&MailAddress> for MailAddress {
    fn from(mail_address: &MailAddress) -> MailAddress {
        mail_address.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_from_every_string_flavor() {
        let owned_name = "Ada".to_owned();
        let owned_addr = "ada@example.com".to_owned();

        let bare: MailAddress = "ada@example.com".into();
        assert_eq!(bare, MailAddress::new("ada@example.com"));

        let from_ref_string: MailAddress = (&owned_addr).into();
        assert_eq!(from_ref_string, bare);

        let expected = MailAddress::named("Ada", "ada@example.com");
        let strs: MailAddress = ("Ada", "ada@example.com").into();
        let mixed: MailAddress = (&owned_name, "ada@example.com").into();
        let refs: MailAddress = (&owned_name, &owned_addr).into();
        assert_eq!(strs, expected);
        assert_eq!(mixed, expected);
        assert_eq!(refs, expected);
    }
}
