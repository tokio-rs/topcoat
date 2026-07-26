//! Mail: declaring a [`Mail`] with the [`mail!`] macro or [`MailBuilder`],
//! and delivering it with [`send`] through the [`Transport`] registered in
//! the app's [`MailConfig`].

pub use topcoat_mail::*;
pub use topcoat_mail_macro::mail;
