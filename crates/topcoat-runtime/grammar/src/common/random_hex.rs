/// Returns 128 random bits as a string of 32 lowercase hexadecimal digits.
///
/// # Panics
///
/// Panics if the operating system's random source is unavailable.
#[must_use]
pub fn random_hex() -> String {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).expect("the operating system's random source is unavailable");
    format!("{:032x}", u128::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_32_lowercase_hex_digits() {
        let hex = random_hex();
        assert_eq!(hex.len(), 32);
        assert!(
            hex.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
    }

    #[test]
    fn differs_between_draws() {
        assert_ne!(random_hex(), random_hex());
    }
}
