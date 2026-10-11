//! The word list: the one built into the binary, or a file named on the command line.
//!
//! The built-in list is embedded with `include_str!`, so the binary runs from any directory. It
//! lives inside the crate (`data/words.txt`) so that the crate stays self-contained when packaged.

use std::fs::File;
use std::io::BufReader;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use kwest_core::{WordBank, WordBankError};
use thiserror::Error;

/// Built-in word list, one word per line.
const WORDS: &str = include_str!("../../../data/words.txt");
/// Letters per word. Words of other lengths in a list are skipped. `unwrap` in a `const` runs at
/// compile time, so it can't panic when the game runs.
pub const WORD_LENGTH: NonZeroUsize = NonZeroUsize::new(5).unwrap();

/// A word list that could not be used. It shows as where the list came from, so the full
/// message reads like `words.txt: failed to read word list: No such file or directory`.
#[derive(Debug, Error)]
pub enum WordListError {
    /// The built-in list is broken. The tests load it, so this means a bad build.
    #[error("built-in word list")]
    BuiltIn {
        /// Why it could not be used.
        source: WordBankError,
    },
    /// The file named on the command line could not be read or has no five-letter words.
    #[error("{}", .path.display())]
    File {
        /// File given on the command line.
        path: PathBuf,
        /// Why it could not be used.
        source: WordBankError,
    },
}

/// Build the word bank from the file, or from the built-in list if there is none.
///
/// # Errors
///
/// Will return `WordListError::File` if the file cannot be read or has no five-letter words, and
/// `WordListError::BuiltIn` if the built-in list is broken.
pub fn load_bank(file: Option<&Path>) -> Result<WordBank, WordListError> {
    let Some(path) = file else {
        return WordBank::from_words(WORDS.lines(), WORD_LENGTH)
            .map_err(|source| WordListError::BuiltIn { source });
    };
    // A closure lets `?` turn the `io::Error` from `open` into a `WordBankError`, through its
    // `From` implementation, so both steps fail with the same type.
    let load = || -> Result<WordBank, WordBankError> {
        let file = File::open(path)?;
        WordBank::from_reader(BufReader::new(file), WORD_LENGTH)
    };
    load().map_err(|source| WordListError::File {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    /// Write a word list to a new temporary file. The file is deleted when the returned value is
    /// dropped, at the end of the test, even if an assertion fails first: a panic unwinds the
    /// stack, which drops the test's local variables.
    fn word_file(contents: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        file
    }

    #[test]
    fn built_in_list_loads() {
        let bank = load_bank(None).unwrap();
        assert_eq!(bank.word_length(), WORD_LENGTH.get());
        assert!(bank.contains("CRANE"));
    }

    #[test]
    fn word_file_keeps_only_five_letter_words() {
        let file = word_file("Crane\nslate\nfox\nplanet\n");
        let bank = load_bank(Some(file.path())).unwrap();
        assert!(bank.contains("CRANE"));
        assert!(bank.contains("SLATE"));
        assert!(!bank.contains("FOX"));
    }

    #[test]
    fn missing_file_names_the_path() {
        // A fresh, empty directory, so the file surely doesn't exist. It is deleted on drop too.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing.txt");
        let error = load_bank(Some(&path)).unwrap_err();
        assert_eq!(error.to_string(), path.display().to_string());
        assert!(matches!(
            error,
            WordListError::File {
                source: WordBankError::Io { .. },
                ..
            }
        ));
    }

    #[test]
    fn file_without_five_letter_words_is_an_error() {
        let file = word_file("fox\ncat\n");
        let error = load_bank(Some(file.path())).unwrap_err();
        assert!(matches!(
            error,
            WordListError::File {
                source: WordBankError::Empty { word_length: 5 },
                ..
            }
        ));
    }
}
