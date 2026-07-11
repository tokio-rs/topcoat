//! A small `const` [FNV-1a] hash.
//!
//! This is not a cryptographic hash; it exists to fold a handful of build
//! inputs (crate names, paths, options, font settings, ...) into a compact,
//! stable id at compile time, so derived URLs and identifiers stay
//! cache-friendly and collision-free across builds.
//!
//! [FNV-1a]: https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function

/// The FNV-1a 64-bit offset basis: the starting state of a fresh hash.
pub const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// Hash `bytes` with FNV-1a, starting from the [offset basis](OFFSET).
///
/// Equivalent to [`hash_continue(OFFSET, bytes)`](hash_continue).
#[must_use]
pub const fn hash(bytes: &[u8]) -> u64 {
    hash_continue(OFFSET, bytes)
}

/// Fold `bytes` into the running hash `h`, continuing an FNV-1a hash.
///
/// Chain calls to hash a sequence of byte runs; separate runs whose boundaries
/// matter with a delimiter so distinct inputs cannot collide by concatenation.
#[must_use]
pub const fn hash_continue(mut h: u64, bytes: &[u8]) -> u64 {
    let mut i = 0;
    while i < bytes.len() {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
        i += 1;
    }
    h
}
