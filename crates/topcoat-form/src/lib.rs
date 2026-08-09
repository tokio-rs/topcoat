#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "router")]
mod client;
#[cfg(feature = "view")]
mod components;
mod control;
mod data;
mod env;
mod field;
#[cfg(feature = "router")]
mod form;
mod result;
mod schema;
mod validator;
mod value;

#[cfg(feature = "router")]
pub use client::*;
#[cfg(feature = "view")]
pub use components::*;
pub use control::*;
pub use data::*;
pub use env::*;
pub use field::*;
#[cfg(feature = "router")]
pub use form::*;
pub use result::*;
pub use schema::*;
pub use topcoat_form_macro::ValidationData;
#[cfg(feature = "router")]
pub use topcoat_form_macro::{
    ClientForm, FormSchema, ValidForm, form_group_handlers, form_validation_handlers,
};
pub use validator::*;
pub use value::*;
