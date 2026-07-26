/// How much of the stream to keep between chunks. A progress render is far
/// shorter than this, so a count split across a chunk boundary is completed
/// by the next chunk before the split half is trimmed away.
const TAIL_KEEP: usize = 128;

/// Trim the tail once it grows past this, bounding it however cargo chops
/// its writes.
const TAIL_LIMIT: usize = 512;

/// Incremental scanner for cargo's build progress.
///
/// Cargo's progress bar renders contain a `current/total` unit count
/// (`Building [===>  ] 12/34: app`). The scanner is fed the raw stderr
/// stream chunk by chunk and reports the newest count each time it changes,
/// keeping a small tail of the stream so a count split across two chunks is
/// still seen whole.
#[derive(Default)]
pub(super) struct ProgressScanner {
    tail: Vec<u8>,
    last: Option<(u64, u64)>,
}

impl ProgressScanner {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Feed the next chunk of stderr. Returns the newest `(current, total)`
    /// count in the stream when it differs from the last one returned.
    pub(super) fn push(&mut self, chunk: &[u8]) -> Option<(u64, u64)> {
        self.tail.extend_from_slice(chunk);
        let progress = last_count(&self.tail);
        if self.tail.len() > TAIL_LIMIT {
            let drain_to = self.tail.len() - TAIL_KEEP;
            self.tail.drain(..drain_to);
        }
        if progress.is_some() && progress != self.last {
            self.last = progress;
            return progress;
        }
        None
    }
}

/// The last `<current>/<total>` pair of integers in `bytes` with
/// `current <= total`, which in cargo's stderr is the progress bar's unit
/// count.
fn last_count(bytes: &[u8]) -> Option<(u64, u64)> {
    let mut last = None;
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let mid = i;
        if i >= bytes.len() || bytes[i] != b'/' {
            continue;
        }
        i += 1;
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            continue;
        }
        let t_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let cur: Option<u64> = std::str::from_utf8(&bytes[start..mid])
            .ok()
            .and_then(|s| s.parse().ok());
        let total: Option<u64> = std::str::from_utf8(&bytes[t_start..i])
            .ok()
            .and_then(|s| s.parse().ok());
        if let (Some(c), Some(t)) = (cur, total)
            && c <= t
            && t > 0
        {
            last = Some((c, t));
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_last_count_in_a_chunk() {
        let mut scanner = ProgressScanner::new();
        assert_eq!(
            scanner.push(b"   Building [=>    ] 1/34: a\r   Building [==>   ] 2/34: b\r"),
            Some((2, 34))
        );
    }

    #[test]
    fn an_unchanged_count_is_reported_once() {
        let mut scanner = ProgressScanner::new();
        assert_eq!(scanner.push(b" 3/9: app\r"), Some((3, 9)));
        assert_eq!(scanner.push(b" 3/9: app(build)\r"), None);
        assert_eq!(scanner.push(b" 4/9: app\r"), Some((4, 9)));
    }

    #[test]
    fn a_count_split_across_chunks_is_seen_whole() {
        let mut scanner = ProgressScanner::new();
        assert_eq!(scanner.push(b"   Building [===>  ] 12"), None);
        assert_eq!(scanner.push(b"/34: app\r"), Some((12, 34)));
    }

    #[test]
    fn pairs_that_are_not_counts_are_ignored() {
        let mut scanner = ProgressScanner::new();
        // A count never exceeds its total, so a date like 2026/07 or a
        // truncated render like 5/3 must not read as progress.
        assert_eq!(scanner.push(b" checked out 2026/07 "), None);
        assert_eq!(scanner.push(b" 5/3 "), None);
        assert_eq!(scanner.push(b" 0/0 "), None);
    }
}
