//! The canonical form of letters and words: uppercase ASCII.
//!
//! The engine converts its input with these functions as it comes in, so inside the engine a letter
//! is always `'A'..='Z'` and comparing letters or words is plain equality. Words are also trimmed
//! of surrounding whitespace. Only ASCII letters are valid; anything else is refused rather than
//! converted.

/// Convert a character to canonical form: uppercase.
pub(crate) fn character(c: char) -> char {
    c.to_ascii_uppercase()
}

/// Convert a word to canonical form: trimmed and uppercase.
pub(crate) fn word(w: &str) -> String {
    w.trim().to_ascii_uppercase()
}

/// Is the character a valid letter? Only ASCII letters are.
pub(crate) fn is_valid(c: char) -> bool {
    c.is_ascii_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_is_converted_to_uppercase() {
        assert_eq!(character('a'), 'A');
        assert_eq!(character('A'), 'A');
    }

    #[test]
    fn word_is_converted_to_uppercase() {
        assert_eq!(word("a"), "A");
        assert_eq!(word("Ab"), "AB");
        assert_eq!(word("\tAbC "), "ABC");
    }

    #[test]
    fn only_ascii_alphabet_is_valid() {
        assert!(is_valid('a'));
        assert!(is_valid('A'));
        assert!(!is_valid('-'));
    }
}
