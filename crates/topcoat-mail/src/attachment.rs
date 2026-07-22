//! Files carried by a mail.

/// A file carried by a mail, either as a downloadable attachment or as
/// inline content referenced from the HTML body.
///
/// Downloadable attachments ([`Attachment::new`]) are presented to the
/// recipient as files. Inline attachments ([`Attachment::inline`]) are
/// addressed from the HTML body by content id -- `<img src="cid:logo">`
/// displays the inline attachment with content id `logo`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    disposition: Disposition,
    content_type: String,
    content: Vec<u8>,
}

impl Attachment {
    /// A downloadable attachment presented with the given filename.
    #[must_use]
    pub fn new(
        filename: impl Into<String>,
        content_type: impl Into<String>,
        content: impl Into<Vec<u8>>,
    ) -> Attachment {
        Attachment {
            disposition: Disposition::Attached {
                filename: filename.into(),
            },
            content_type: content_type.into(),
            content: content.into(),
        }
    }

    /// An inline attachment the HTML body references as `cid:{content_id}`.
    #[must_use]
    pub fn inline(
        content_id: impl Into<String>,
        content_type: impl Into<String>,
        content: impl Into<Vec<u8>>,
    ) -> Attachment {
        Attachment {
            disposition: Disposition::Inline {
                content_id: content_id.into(),
            },
            content_type: content_type.into(),
            content: content.into(),
        }
    }

    /// The filename of a downloadable attachment, or `None` for an inline
    /// one.
    #[must_use]
    pub fn filename(&self) -> Option<&str> {
        match &self.disposition {
            Disposition::Attached { filename } => Some(filename),
            Disposition::Inline { .. } => None,
        }
    }

    /// The content id the HTML body references an inline attachment by, or
    /// `None` for a downloadable one.
    #[must_use]
    pub fn content_id(&self) -> Option<&str> {
        match &self.disposition {
            Disposition::Attached { .. } => None,
            Disposition::Inline { content_id } => Some(content_id),
        }
    }

    /// The declared MIME type of the content.
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// The content bytes.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

/// How an attachment is presented to the recipient.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Disposition {
    /// A file the recipient downloads.
    Attached { filename: String },
    /// Content the HTML body embeds by content id.
    Inline { content_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_downloadable_from_inline() {
        let file = Attachment::new("invoice.pdf", "application/pdf", b"%PDF-");
        assert_eq!(file.filename(), Some("invoice.pdf"));
        assert_eq!(file.content_id(), None);
        assert_eq!(file.content_type(), "application/pdf");
        assert_eq!(file.content(), b"%PDF-");

        let logo = Attachment::inline("logo", "image/png", b"\x89PNG");
        assert_eq!(logo.filename(), None);
        assert_eq!(logo.content_id(), Some("logo"));
        assert_eq!(logo.content_type(), "image/png");
    }
}
