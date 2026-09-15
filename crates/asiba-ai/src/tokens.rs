const CHARS_PER_TOKEN_NUMERATOR: usize = 3;
const CHARS_PER_TOKEN_DENOMINATOR: usize = 7;

pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() * CHARS_PER_TOKEN_NUMERATOR / CHARS_PER_TOKEN_DENOMINATOR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_uses_about_two_and_a_third_chars_per_token() {
        assert_eq!(estimate_tokens(&"a".repeat(700)), 300);
        assert_eq!(estimate_tokens(""), 0);
    }
}
