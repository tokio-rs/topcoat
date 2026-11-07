// Show router links as plain text when the router feature is disabled.
#![cfg_attr(not(feature = "router"), allow(rustdoc::broken_intra_doc_links))]

/// A context marker indicating that the browser runtime has been set up.
///
/// [`RuntimeLayer`](crate::RuntimeLayer) adds it to the context of the
/// requests it passes on.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeSetup;
