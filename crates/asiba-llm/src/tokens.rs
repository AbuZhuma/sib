const CHARS_PER_TOKEN_NUMERATOR: usize = 2;
const CHARS_PER_TOKEN_DENOMINATOR: usize = 7;

pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() * CHARS_PER_TOKEN_NUMERATOR / CHARS_PER_TOKEN_DENOMINATOR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_uses_three_and_half_chars_per_token() {
        assert_eq!(estimate_tokens(&"a".repeat(350)), 100);
        assert_eq!(estimate_tokens(""), 0);
    }
}
