//! Derivation of the ids that name shard and procedure endpoints.
//!
//! The `#[shard]` and `#[procedure]` macros cannot see the module a
//! declaration lives in, because expansion happens before name resolution.
//! They instead emit `module_path!()` into the generated code and fold it
//! here, in the declaring crate, the same way
//! [`AssetId::new`](topcoat_asset::AssetId::new) folds an asset declaration.
//!
//! The key is the declaring crate, the module path, and the item name. Two
//! items cannot share a name inside one module, so the key is collision-free
//! for declarations at module scope, and it stays the same across rebuilds of
//! unchanged source.

use topcoat_core::fnv1a;

/// Length of the ASCII hex string an endpoint id renders as.
pub const ENDPOINT_ID_LEN: usize = 16;

/// Folds the declaration site of a shard or procedure into one `u64`.
///
/// `crate_name` comes from `CARGO_CRATE_NAME`, `module_path` from
/// `module_path!()`, and `ident` from the name of the annotated function.
#[must_use]
pub const fn endpoint_id_hash(crate_name: &str, module_path: &str, ident: &str) -> u64 {
    let mut h = fnv1a::hash(crate_name.as_bytes());
    h = fnv1a::hash_continue(h, b"\0");
    h = fnv1a::hash_continue(h, module_path.as_bytes());
    h = fnv1a::hash_continue(h, b"\0");
    h = fnv1a::hash_continue(h, ident.as_bytes());
    h
}

/// Renders `hash` as lowercase, big-endian ASCII hex.
///
/// The result is valid UTF-8 and safe in a URL path segment, so generated
/// code turns it into the `&'static str` a shard or procedure id wraps.
#[must_use]
pub const fn endpoint_id_hex(hash: u64) -> [u8; ENDPOINT_ID_LEN] {
    const DIGITS: [u8; 16] = *b"0123456789abcdef";

    let bytes = hash.to_be_bytes();
    let mut out = [b'0'; ENDPOINT_ID_LEN];
    let mut i = 0;
    while i < bytes.len() {
        out[i * 2] = DIGITS[(bytes[i] >> 4) as usize];
        out[i * 2 + 1] = DIGITS[(bytes[i] & 0x0f) as usize];
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(crate_name: &str, module_path: &str, ident: &str) -> String {
        let bytes = endpoint_id_hex(endpoint_id_hash(crate_name, module_path, ident));
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[test]
    fn pins_the_id_of_a_known_declaration() {
        assert_eq!(id("my_app", "my_app::widgets", "rows"), "7fc28631a7c30011");
    }

    #[test]
    fn separates_the_same_name_in_two_modules() {
        assert_ne!(
            id("my_app", "my_app::a", "rows"),
            id("my_app", "my_app::b", "rows")
        );
    }

    #[test]
    fn separates_the_same_declaration_in_two_crates() {
        assert_ne!(
            id("my_app", "shared::widgets", "rows"),
            id("other_app", "shared::widgets", "rows")
        );
    }

    #[test]
    fn renders_hex_of_a_fixed_width() {
        assert_eq!(id("a", "b", "c").len(), ENDPOINT_ID_LEN);
        assert_eq!(
            String::from_utf8(endpoint_id_hex(0).to_vec()).unwrap(),
            "0000000000000000"
        );
        assert_eq!(
            String::from_utf8(endpoint_id_hex(u64::MAX).to_vec()).unwrap(),
            "ffffffffffffffff"
        );
    }
}
