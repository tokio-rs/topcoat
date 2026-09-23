//! Files carried by a mail.

/// A file sent with a mail, either as a regular attachment or as inline
/// content of the HTML body.
///
/// A regular attachment ([`Attachment::new`]) is shown to the recipient as a
/// file to download. An inline attachment ([`Attachment::inline`]) is shown
/// inside the HTML body, which references it by content id. For example,
/// `<img src="cid:logo">` displays the inline attachment with the content id
/// `logo`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    disposition: Disposition,
    content_type: String,
    content: Vec<u8>,
}

impl Attachment {
    /// Creates a regular attachment that the recipient sees as a file named
    /// `filename`. `content_type` is a MIME type, such as
    /// `"application/pdf"`.
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

    /// Creates an inline attachment that the HTML body references with the
    /// URL `cid:{content_id}`. `content_type` is a MIME type, such as
    /// `"image/png"`.
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

    /// The file name of a regular attachment, or `None` for an inline one.
    #[must_use]
    pub fn filename(&self) -> Option<&str> {
        match &self.disposition {
            Disposition::Attached { filename } => Some(filename),
            Disposition::Inline { .. } => None,
        }
    }

    /// The content id of an inline attachment, or `None` for a regular one.
    #[must_use]
    pub fn content_id(&self) -> Option<&str> {
        match &self.disposition {
            Disposition::Attached { .. } => None,
            Disposition::Inline { content_id } => Some(content_id),
        }
    }

    /// The MIME type of the content.
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

/// Conversion into a list of attachments.
///
/// Implemented for a single [`Attachment`] and for collections of them. The
/// `attachments` field of the `mail!` macro accepts any value of this
/// trait.
pub trait IntoAttachments {
    /// Converts into attachments.
    fn into_attachments(self) -> Vec<Attachment>;
}

impl IntoAttachments for Attachment {
    fn into_attachments(self) -> Vec<Attachment> {
        vec![self]
    }
}

impl IntoAttachments for &Attachment {
    fn into_attachments(self) -> Vec<Attachment> {
        vec![self.clone()]
    }
}

impl IntoAttachments for Vec<Attachment> {
    fn into_attachments(self) -> Vec<Attachment> {
        self
    }
}

impl<const N: usize> IntoAttachments for [Attachment; N] {
    fn into_attachments(self) -> Vec<Attachment> {
        self.into()
    }
}

impl IntoAttachments for &[Attachment] {
    fn into_attachments(self) -> Vec<Attachment> {
        self.to_vec()
    }
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
