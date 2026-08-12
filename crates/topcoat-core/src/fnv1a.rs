//! A small `const` [FNV-1a] hasher.
//!
//! This is not a cryptographic hash; it exists to fold a handful of build
//! inputs (crate names, paths, options, font settings, ...) into a compact,
//! stable id at compile time, so derived URLs and identifiers stay
//! cache-friendly and collision-free across builds.
//!
//! [FNV-1a]: https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function

/// A `const` [FNV-1a] hasher over a `u64` or `u128` state.
///
/// [`new`](Self::new) starts a hash at the offset basis, [`write`](Self::write)
/// folds a run of bytes in, and [`finish`](Self::finish) takes the value out.
/// Every step moves the hasher, so a running hash threads through a chain of
/// calls and cannot fork by accident. Separate runs whose boundaries matter
/// with a delimiter so distinct inputs cannot collide by concatenation.
///
/// ```
/// use topcoat_core::fnv1a::Fnv1a;
///
/// const ID: u64 = Fnv1a::<u64>::new()
///     .write(b"my-crate")
///     .write(b"\0")
///     .write(b"src/lib.rs")
///     .finish();
/// ```
///
/// [FNV-1a]: https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function
pub struct Fnv1a<T>(T);

/// Implements the hasher for one state width.
///
/// The methods are inherent per width rather than trait-provided because
/// trait methods cannot be `const` on stable Rust.
macro_rules! fnv1a_impl {
    ($ty:ty, $offset:literal, $prime:literal) => {
        impl Fnv1a<$ty> {
            /// Creates a fresh hasher, starting at the offset basis.
            #[must_use]
            pub const fn new() -> Self {
                Self($offset)
            }

            /// Folds `bytes` into the running hash.
            #[must_use]
            pub const fn write(mut self, bytes: &[u8]) -> Self {
                let mut i = 0;
                while i < bytes.len() {
                    self.0 ^= bytes[i] as $ty;
                    self.0 = self.0.wrapping_mul($prime);
                    i += 1;
                }
                self
            }

            /// Returns the hash of everything written so far.
            #[must_use]
            pub const fn finish(self) -> $ty {
                self.0
            }
        }

        impl Default for Fnv1a<$ty> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

fnv1a_impl!(u64, 0xcbf2_9ce4_8422_2325_u64, 0x0100_0000_01b3_u64);
fnv1a_impl!(
    u128,
    0x6c62_272e_07bb_0142_62b8_2175_6295_c58d_u128,
    0x0000_0000_0100_0000_0000_0000_0000_013b_u128
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_the_offset_basis() {
        assert_eq!(Fnv1a::<u64>::new().finish(), 0xcbf2_9ce4_8422_2325);
        assert_eq!(
            Fnv1a::<u128>::new().finish(),
            0x6c62_272e_07bb_0142_62b8_2175_6295_c58d,
        );
    }

    #[test]
    fn matches_the_reference_vectors() {
        // Test vectors from the FNV reference material.
        assert_eq!(
            Fnv1a::<u64>::new().write(b"a").finish(),
            0xaf63_dc4c_8601_ec8c,
        );
        assert_eq!(
            Fnv1a::<u64>::new().write(b"foobar").finish(),
            0x8594_4171_f739_67e8,
        );
    }

    #[test]
    fn writes_chain() {
        assert_eq!(
            Fnv1a::<u64>::new().write(b"foo").write(b"bar").finish(),
            Fnv1a::<u64>::new().write(b"foobar").finish(),
        );
        assert_eq!(
            Fnv1a::<u128>::new().write(b"foo").write(b"bar").finish(),
            Fnv1a::<u128>::new().write(b"foobar").finish(),
        );
    }

    #[test]
    fn distinct_inputs_hash_distinctly() {
        assert_ne!(
            Fnv1a::<u128>::new().write(b"a").finish(),
            Fnv1a::<u128>::new().write(b"b").finish(),
        );
        assert_ne!(
            Fnv1a::<u128>::new().write(b"a").finish(),
            Fnv1a::<u128>::new().finish(),
        );
    }

    #[test]
    fn hashes_in_const_context() {
        const HASH: u64 = Fnv1a::<u64>::new().write(b"const").finish();
        assert_eq!(HASH, Fnv1a::<u64>::new().write(b"const").finish());
    }
}
