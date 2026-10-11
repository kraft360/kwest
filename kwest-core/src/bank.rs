//! The word bank: every word a game accepts, and the pool its secret is drawn from.
//!
//! Words are converted to canonical form when the bank is built, and words of the wrong length or
//! with characters other than letters are skipped, so a raw word list can be passed in as it is.
//! The words are then sorted and deduplicated, which makes [`WordBank::contains`] a binary search.
//! A bank is never empty, so [`WordBank::random_word`] always has a word to return.
//!
//! The word length is a [`NonZeroUsize`], so a bank of zero-letter words can't be asked for: the
//! compiler rejects it instead of a runtime check.

use std::io::BufRead;
use std::num::NonZeroUsize;

use crate::canonical;
use crate::errors::WordBankError;
use rand::{Rng, RngExt};

/// A non-empty, sorted list of canonical words, all of the same length.
#[derive(Clone, Debug)]
pub struct WordBank {
    /// Canonical words, sorted and without duplicates.
    words: Vec<String>,
    /// Number of letters in every word.
    word_length: NonZeroUsize,
}

impl WordBank {
    /// Create a word bank from a list of words in any case.
    /// Words of another length, or with characters other than ASCII letters, are skipped.
    ///
    /// # Errors
    ///
    /// Will return `WordBankError::Empty` if no valid word is left.
    pub fn from_words(
        words: impl IntoIterator<Item = impl AsRef<str>>,
        word_length: NonZeroUsize,
    ) -> Result<Self, WordBankError> {
        let mut words = words
            .into_iter()
            .map(|s| canonical::word(s.as_ref()))
            .filter(|s| Self::is_valid(s, word_length))
            .collect::<Vec<String>>();
        // Sorted and deduplicated, so that `contains` can use a binary search.
        words.sort_unstable();
        words.dedup();
        if words.is_empty() {
            Err(WordBankError::Empty {
                word_length: word_length.get(),
            })
        } else {
            Ok(Self { words, word_length })
        }
    }

    /// Create a word bank from a reader with one word per line, as for `from_words`.
    ///
    /// # Errors
    ///
    /// Will return `WordBankError::Empty` if no valid word is left.
    /// Will return `WordBankError::Io` if reading fails.
    pub fn from_reader(
        reader: impl BufRead,
        word_length: NonZeroUsize,
    ) -> Result<Self, WordBankError> {
        let words = reader.lines().collect::<std::io::Result<Vec<_>>>()?;
        Self::from_words(words, word_length)
    }

    /// Does the canonical word have the right length and only valid letters?
    fn is_valid(word: &str, word_length: NonZeroUsize) -> bool {
        word.chars().count() == word_length.get() && word.chars().all(canonical::is_valid)
    }

    /// Number of words in the bank.
    fn len(&self) -> usize {
        self.words.len()
    }

    /// Get the number of letters in every word. It is never zero; it is returned as a plain
    /// `usize` because callers count and index with it.
    #[must_use]
    pub fn word_length(&self) -> usize {
        self.word_length.get()
    }

    /// Pick a word at random with the given random number generator.
    #[must_use]
    pub fn random_word(&self, generator: &mut impl Rng) -> &str {
        // The bank is never empty, so the range is never empty (`random_range` would panic) and
        // the index is always in bounds.
        let index = generator.random_range(0..self.len());
        self.words[index].as_str()
    }

    /// Is the word in the bank? It is converted to canonical form first, so case and surrounding
    /// whitespace don't matter.
    #[must_use]
    pub fn contains(&self, word: &str) -> bool {
        self.words.binary_search(&canonical::word(word)).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Word length in every test. `unwrap` in a `const` runs at compile time.
    const FIVE: NonZeroUsize = NonZeroUsize::new(5).unwrap();

    #[test]
    fn skips_words_of_wrong_lengths() {
        let words = vec!["Hello", "Big", "World", "Global"];
        let bank = WordBank::from_words(words, FIVE).unwrap();
        assert_eq!(bank.len(), 2, "Bank size with filter");
        assert!(bank.contains("HELLO"));
        assert!(bank.contains("WORLD"));
    }

    #[test]
    fn skips_words_with_non_ascii_alphabets() {
        let words = vec!["Hello", "He-lo", "World", "Cafés"];
        let bank = WordBank::from_words(words, FIVE).unwrap();
        assert_eq!(bank.len(), 2, "Bank size with filter");
        assert!(bank.contains("HELLO"));
        assert!(bank.contains("WORLD"));
    }

    #[test]
    fn normalizes_and_dedupes() {
        let content = " Hello\t\n\tBig \nWorld\nGlobal\nHello\nWorld";
        let bank = WordBank::from_reader(content.as_bytes(), FIVE).unwrap();
        assert_eq!(bank.len(), 2, "Bank size with filter");
        assert!(bank.contains("HELLO"));
        assert!(bank.contains("WORLD"));
    }

    #[test]
    fn empty_list_returns_error() {
        assert!(matches!(
            WordBank::from_words([""], FIVE),
            Err(WordBankError::Empty { word_length: 5 })
        ));
    }

    #[test]
    fn random_word_is_from_the_bank() {
        let words = vec!["Hello", "Big", "World"];
        let bank = WordBank::from_words(words, FIVE).unwrap();
        let mut generator = rand::rng();
        let random_word = bank.random_word(&mut generator);
        assert!(["HELLO", "WORLD"].contains(&random_word));
    }

    #[test]
    fn single_word_bank_always_returns_it() {
        let words = vec!["Hello"];
        let bank = WordBank::from_words(words, FIVE).unwrap();
        let mut generator = rand::rng();
        let random_word = bank.random_word(&mut generator);
        assert_eq!(random_word, "HELLO");
    }

    #[test]
    fn contains_normalizes_before_check() {
        let words = vec!["Hello"];
        let bank = WordBank::from_words(words, FIVE).unwrap();
        assert!(bank.contains("\tHello "));
        assert!(!bank.contains(" world "));
        assert!(!bank.contains("worl"));
        assert!(!bank.contains("he-ll"));
    }
}
