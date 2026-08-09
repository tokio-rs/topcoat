use crate::fnv1a::Fnv1a;

/// A value that tells repetitions of one invocation site apart.
///
/// A key folds itself into the identity hash through the tagged writes on
/// [`KeyHasher`]. Two keys derive the same identity exactly when they
/// produce the same sequence of writes, so implementations must be
/// deterministic: equal values write equal sequences, and values meant to
/// be distinct at one site write distinct sequences.
///
/// Implementations exist for the integer primitives, `bool`, `char`,
/// strings, byte slices, references, and tuples of keys. Integers hash by
/// mathematical value, so the same id used at a different width stays the
/// same key. A custom id type implements the trait by writing its
/// identifying parts in order:
///
/// ```
/// use topcoat_core::identity::{IdentityKey, KeyHasher};
///
/// struct UserId(u64);
///
/// impl IdentityKey for UserId {
///     fn write(&self, hasher: KeyHasher) -> KeyHasher {
///         hasher.write_u128(u128::from(self.0))
///     }
/// }
/// ```
pub trait IdentityKey {
    /// Folds this key into the running identity hash.
    #[must_use]
    fn write(&self, hasher: KeyHasher) -> KeyHasher;
}

/// Tag byte starting an unsigned or non-negative integer write.
const TAG_UNSIGNED: u8 = b'u';
/// Tag byte starting a negative integer write.
const TAG_NEGATIVE: u8 = b'i';
/// Tag byte starting a boolean write.
const TAG_BOOL: u8 = b'b';
/// Tag byte starting a character write.
const TAG_CHAR: u8 = b'c';
/// Tag byte starting a string write.
const TAG_STR: u8 = b's';
/// Tag byte starting a raw byte write.
const TAG_BYTES: u8 = b'x';
/// Tag byte starting a tuple frame.
const TAG_TUPLE: u8 = b'(';

/// Terminator ending a string write.
///
/// `0xFF` never appears in UTF-8 encoded text, so it marks the end of a
/// string without escaping.
const STR_END: u8 = 0xFF;

/// The hasher an [`IdentityKey`] folds itself into.
///
/// Wraps the running identity hash during key derivation. Every write is
/// tagged with the kind of data written and is self-delimiting, so keys of
/// different kinds, and sequences of writes with different boundaries,
/// cannot collide by concatenation. The hasher moves through every write,
/// threading through a chain of calls, and only the derivation that created
/// it can take the final value out.
pub struct KeyHasher(Fnv1a<u128>);

impl KeyHasher {
    /// Wraps the running hash of a keyed derivation.
    pub(super) fn new(hash: Fnv1a<u128>) -> Self {
        Self(hash)
    }

    /// Takes the derived hash value out.
    pub(super) fn finish(self) -> u128 {
        self.0.finish()
    }

    /// Writes an unsigned integer by value.
    #[must_use]
    pub fn write_u128(self, value: u128) -> Self {
        Self(self.0.write(&[TAG_UNSIGNED]).write(&value.to_le_bytes()))
    }

    /// Writes a signed integer by mathematical value: a non-negative value
    /// writes exactly like the equal unsigned value.
    #[must_use]
    pub fn write_i128(self, value: i128) -> Self {
        if value >= 0 {
            self.write_u128(value.cast_unsigned())
        } else {
            Self(self.0.write(&[TAG_NEGATIVE]).write(&value.to_le_bytes()))
        }
    }

    /// Writes a boolean.
    #[must_use]
    pub fn write_bool(self, value: bool) -> Self {
        Self(self.0.write(&[TAG_BOOL, u8::from(value)]))
    }

    /// Writes a character by code point.
    #[must_use]
    pub fn write_char(self, value: char) -> Self {
        Self(
            self.0
                .write(&[TAG_CHAR])
                .write(&u32::from(value).to_le_bytes()),
        )
    }

    /// Writes a string.
    #[must_use]
    pub fn write_str(self, value: &str) -> Self {
        Self(
            self.0
                .write(&[TAG_STR])
                .write(value.as_bytes())
                .write(&[STR_END]),
        )
    }

    /// Writes raw bytes, length-prefixed since any byte value can occur in
    /// them.
    #[must_use]
    pub fn write_bytes(self, value: &[u8]) -> Self {
        Self(
            self.0
                .write(&[TAG_BYTES])
                .write(&(value.len() as u64).to_le_bytes())
                .write(value),
        )
    }

    /// Frames a tuple of `len` elements, so a nested tuple never writes the
    /// same sequence as its flattened elements.
    fn tuple(self, len: u8) -> Self {
        Self(self.0.write(&[TAG_TUPLE, len]))
    }
}

impl<K: IdentityKey + ?Sized> IdentityKey for &K {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        (**self).write(hasher)
    }
}

/// Implements the key trait for unsigned integer primitives.
macro_rules! unsigned_key_impl {
    ($($ty:ty),*) => {$(
        impl IdentityKey for $ty {
            fn write(&self, hasher: KeyHasher) -> KeyHasher {
                hasher.write_u128(u128::from(*self))
            }
        }
    )*};
}

unsigned_key_impl!(u8, u16, u32, u64, u128);

impl IdentityKey for usize {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_u128(*self as u128)
    }
}

/// Implements the key trait for signed integer primitives.
macro_rules! signed_key_impl {
    ($($ty:ty),*) => {$(
        impl IdentityKey for $ty {
            fn write(&self, hasher: KeyHasher) -> KeyHasher {
                hasher.write_i128(i128::from(*self))
            }
        }
    )*};
}

signed_key_impl!(i8, i16, i32, i64, i128);

impl IdentityKey for isize {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_i128(*self as i128)
    }
}

impl IdentityKey for bool {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_bool(*self)
    }
}

impl IdentityKey for char {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_char(*self)
    }
}

impl IdentityKey for str {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_str(self)
    }
}

impl IdentityKey for String {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_str(self)
    }
}

impl IdentityKey for [u8] {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_bytes(self)
    }
}

impl<const N: usize> IdentityKey for [u8; N] {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_bytes(self)
    }
}

impl IdentityKey for Vec<u8> {
    fn write(&self, hasher: KeyHasher) -> KeyHasher {
        hasher.write_bytes(self)
    }
}

/// Implements the key trait for one tuple arity.
macro_rules! tuple_key_impl {
    ($len:literal: $(($name:ident, $idx:tt)),+) => {
        impl<$($name: IdentityKey),+> IdentityKey for ($($name,)+) {
            fn write(&self, hasher: KeyHasher) -> KeyHasher {
                let hasher = hasher.tuple($len);
                $(let hasher = self.$idx.write(hasher);)+
                hasher
            }
        }
    };
}

tuple_key_impl!(1: (A, 0));
tuple_key_impl!(2: (A, 0), (B, 1));
tuple_key_impl!(3: (A, 0), (B, 1), (C, 2));
tuple_key_impl!(4: (A, 0), (B, 1), (C, 2), (D, 3));
tuple_key_impl!(5: (A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
tuple_key_impl!(6: (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
tuple_key_impl!(7: (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
tuple_key_impl!(8: (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7));
tuple_key_impl!(9: (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8));
tuple_key_impl!(10: (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9));
tuple_key_impl!(11: (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10));
tuple_key_impl!(12: (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11));

#[cfg(test)]
mod tests {
    use super::*;

    /// Hashes a key standalone, outside any derivation.
    fn hash(key: impl IdentityKey) -> u128 {
        key.write(KeyHasher::new(Fnv1a::<u128>::new())).finish()
    }

    #[test]
    fn integers_hash_by_mathematical_value() {
        assert_eq!(hash(5u8), hash(5u128));
        assert_eq!(hash(5u32), hash(5i32));
        assert_eq!(hash(-5i8), hash(-5i64));
        assert_ne!(hash(5i32), hash(-5i32));
    }

    #[test]
    fn kinds_never_coerce_into_each_other() {
        assert_ne!(hash(1u32), hash("1"));
        assert_ne!(hash('1'), hash("1"));
        assert_ne!(hash('a'), hash(97u32));
        assert_ne!(hash(true), hash(1u32));
        assert_ne!(hash("a"), hash(*b"a"));
    }

    #[test]
    fn strings_hash_by_content() {
        assert_eq!(hash("a"), hash(String::from("a")));
        assert_ne!(hash("a"), hash("b"));
        assert_ne!(hash(""), hash(0u32));
    }

    #[test]
    fn references_hash_like_their_target() {
        assert_eq!(hash(&5u32), hash(5u32));
        assert_eq!(hash(&"a"), hash("a"));
        assert_eq!(hash(b"a".as_slice()), hash(*b"a"));
    }

    #[test]
    fn tuples_frame_their_elements() {
        assert_eq!(hash((1, "a")), hash((1, "a")));
        assert_ne!(hash((1, 2)), hash((2, 1)));
        assert_ne!(hash((1, (2, 3))), hash((1, 2, 3)));
        assert_ne!(hash(("ab", "c")), hash(("a", "bc")));
        assert_ne!(hash((1,)), hash(1));
    }
}
