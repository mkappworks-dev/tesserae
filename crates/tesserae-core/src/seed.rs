//! A World seed from what a Player typed.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeedError {
    OutOfRange,
}

impl std::fmt::Display for SeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("number is outside the seed range")
    }
}

/// `None` for blank text: the caller picks a random seed.
pub fn resolve(text: &str) -> Result<Option<u64>, SeedError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if digits(text) {
        return text
            .parse::<u64>()
            .map(Some)
            .map_err(|_| SeedError::OutOfRange);
    }
    if text.strip_prefix('-').is_some_and(digits) {
        return text
            .parse::<i64>()
            .map(|n| Some(n as u64))
            .map_err(|_| SeedError::OutOfRange);
    }
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(text.as_bytes());
    Ok(Some(u64::from_be_bytes(
        hash[..8].try_into().expect("SHA-256 is 32 bytes"),
    )))
}

/// From the OS random source, for blank seed text and Randomize.
pub fn random() -> u64 {
    getrandom::u64().expect("the OS random source is available")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_text_asks_for_a_random_seed() {
        assert_eq!(resolve(""), Ok(None));
        assert_eq!(resolve(" \t "), Ok(None));
    }

    #[test]
    fn unsigned_decimal_is_the_seed() {
        assert_eq!(resolve("0"), Ok(Some(0)));
        assert_eq!(resolve("  42 "), Ok(Some(42)));
        assert_eq!(resolve("007"), Ok(Some(7)));
        assert_eq!(resolve("18446744073709551615"), Ok(Some(u64::MAX)));
    }

    #[test]
    fn negative_decimal_is_its_twos_complement() {
        assert_eq!(resolve("-1"), Ok(Some(u64::MAX)));
        assert_eq!(resolve("-0"), Ok(Some(0)));
        assert_eq!(resolve("-9223372036854775808"), Ok(Some(1 << 63)));
    }

    #[test]
    fn numbers_outside_both_ranges_are_rejected_not_hashed() {
        assert_eq!(resolve("18446744073709551616"), Err(SeedError::OutOfRange));
        assert_eq!(resolve("-9223372036854775809"), Err(SeedError::OutOfRange));
        assert_eq!(
            resolve("99999999999999999999999999"),
            Err(SeedError::OutOfRange)
        );
    }

    /// Expected values are the first 8 bytes of `shasum -a 256` of the UTF-8 text.
    #[test]
    fn anything_else_is_a_word_hashed_with_sha256() {
        assert_eq!(resolve("Glacier"), Ok(Some(17_202_772_764_494_755_323)));
        assert_eq!(resolve("glacier"), Ok(Some(10_469_079_827_758_943_916)));
        assert_eq!(resolve(" café "), Ok(Some(9_588_020_413_419_552_649)));
        assert_eq!(resolve("1 2"), Ok(Some(17_805_430_819_384_957_993)));
        assert_eq!(resolve("-"), Ok(Some(4_139_898_923_029_504_249)));
        assert_eq!(resolve("+5"), Ok(Some(278_873_215_304_611_275)));
    }
}
