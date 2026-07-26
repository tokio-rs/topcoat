//! The file transport for development.

use std::{path::PathBuf, time::SystemTime};

use time::{OffsetDateTime, macros::format_description};
use topcoat_core::context::Cx;

use crate::{
    Mail, Receipt, SendError, Transport, TransportFuture,
    mime::{self, BccHeader},
};

/// A [`Transport`] that writes each mail as an `.eml` file instead of
/// delivering it, for inspecting mail during development.
///
/// ```no_run
/// use topcoat_mail::FileTransport;
///
/// let transport = FileTransport::new("target/mail");
/// ```
///
/// Files land in the given directory (created on first send). The name
/// carries the UTC send time and the start of the `Message-ID`, like
/// `20260726-143502-1a2b3c4d.eml`, so a directory listing shows sends
/// in order. Any mail client or editor opens the format. Unlike the wire
/// form a delivering transport produces, the file keeps the `Bcc` header,
/// so the full recipient list stays inspectable.
///
/// The file is written with blocking I/O on the calling task, which is fine
/// for development but makes this transport unsuited for production
/// serving.
#[derive(Clone, Debug)]
pub struct FileTransport {
    directory: PathBuf,
}

impl FileTransport {
    /// Creates a transport writing into `directory`.
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> FileTransport {
        FileTransport {
            directory: directory.into(),
        }
    }

    /// The directory the transport writes into.
    #[must_use]
    pub fn directory(&self) -> &std::path::Path {
        &self.directory
    }
}

impl Transport for FileTransport {
    fn send<'a>(&'a self, cx: &'a Cx, mail: Mail) -> TransportFuture<'a> {
        Box::pin(async move {
            let message = mime::message(cx, &mail, BccHeader::Kept)?;
            let message_id = mime::message_id(&message);

            let path = self.directory.join(format!(
                "{}-{}.eml",
                timestamp(SystemTime::now()),
                filename_safe(&message_id)
            ));

            std::fs::create_dir_all(&self.directory).map_err(SendError::delivery)?;
            std::fs::write(&path, message.formatted()).map_err(SendError::delivery)?;

            Ok(Receipt::new(message_id))
        })
    }
}

/// Formats the UTC send time as a readable, filename-safe
/// `20260726-143502` prefix.
fn timestamp(time: SystemTime) -> String {
    let format = format_description!("[year][month][day]-[hour][minute][second]");
    OffsetDateTime::from(time)
        .format(format)
        .unwrap_or_default()
}

/// Reduces a `Message-ID` to a short prefix of filename-safe characters.
fn filename_safe(message_id: &str) -> String {
    message_id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicU32, Ordering},
        time::{Duration, UNIX_EPOCH},
    };

    use super::*;
    use crate::Mailbox;

    fn temp_directory() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        std::env::temp_dir().join(format!(
            "topcoat-mail-file-transport-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[tokio::test]
    async fn writes_the_mail_as_an_eml_file() {
        let directory = temp_directory();
        let transport = FileTransport::new(&directory);

        let mail = Mail::builder()
            .from(Mailbox::new("ada@example.com").unwrap())
            .to([Mailbox::new("bob@example.com").unwrap()])
            .bcc([Mailbox::new("dan@example.com").unwrap()])
            .subject("Hello")
            .text("Hi Bob")
            .build();
        let receipt = transport.send(&Cx::default(), mail).await.unwrap();

        let entries: Vec<_> = std::fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].extension().unwrap(), "eml");

        let filename = entries[0].file_name().unwrap().to_str().unwrap();
        assert!(filename.starts_with(&timestamp(SystemTime::now())[..9]));

        let wire = std::fs::read_to_string(&entries[0]).unwrap();
        assert!(wire.contains("Subject: Hello\r\n"));
        assert!(wire.contains("Bcc: dan@example.com\r\n"));
        assert!(wire.contains(receipt.message_id()));

        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn timestamps_format_the_utc_send_time() {
        let leap_day = UNIX_EPOCH + Duration::from_secs(1_709_251_199);
        assert_eq!(timestamp(leap_day), "20240229-235959");
    }

    #[test]
    fn filenames_keep_a_short_safe_message_id_prefix() {
        assert_eq!(filename_safe("<1a2b.3c4d5e@host.example>"), "1a2b3c4d");
    }
}
