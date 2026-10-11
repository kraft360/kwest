//! Errors returned by the engine.
//!
//! - [`WordBankError`]: a word bank could not be built. This happens once, at startup, and a
//!   frontend usually reports it and exits.
//! - [`RejectionError`]: a move was refused, and the game is unchanged. These are routine rather
//!   than failures: they happen whenever the player presses a key that doesn't apply, and the
//!   frontend decides which ones are worth showing.
//!
//! The `Display` text of these errors is written for developers. A frontend should word its own
//! messages for players.

use thiserror::Error;

/// Errors building a word bank.
#[derive(Debug, Error)]
pub enum WordBankError {
    /// Reading the word list failed.
    #[error("failed to read word list")]
    Io {
        /// What went wrong while reading.
        #[from]
        source: std::io::Error,
    },
    /// The word list has no valid word of the requested length.
    #[error("no words of length {word_length} found")]
    Empty {
        /// Requested word length.
        word_length: usize,
    },
}

/// Reasons a game move was refused. A refused move leaves the game unchanged.
///
/// A variant that refuses a whole guess names it (`attempt`), so a frontend can mark that row
/// without asking the game.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum RejectionError {
    /// The game is already over.
    #[error("the game is over")]
    GameOver,
    /// The character is not a valid letter.
    #[error("{ch:?} is not a letter")]
    InvalidCharacter {
        /// Character that was refused.
        ch: char,
    },
    /// The active guess is already full.
    #[error("the guess already has {maximum} letters")]
    LengthExceededMaximum {
        /// Most letters a guess can have: the word length.
        maximum: usize,
    },
    /// The active guess has no letter to delete.
    #[error("the guess has no letters to delete")]
    EmptyGuess,
    /// The guess was submitted before it had every letter.
    #[error("the guess has {letters} of {word_length} letters")]
    IncompleteGuess {
        /// Number of letters typed.
        letters: usize,
        /// Number of letters a guess needs: the word length.
        word_length: usize,
        /// Which guess it was, from 0.
        attempt: usize,
    },
    /// The guess is not in the word bank.
    #[error("{word} is not in the word bank")]
    NotInWordBank {
        /// Guess that was refused.
        word: String,
        /// Which guess it was, from 0.
        attempt: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_bank_messages_read_as_sentences() {
        assert_eq!(
            WordBankError::Empty { word_length: 5 }.to_string(),
            "no words of length 5 found"
        );
    }

    #[test]
    fn rejection_messages_read_as_sentences() {
        let cases = [
            (RejectionError::GameOver, "the game is over"),
            (
                RejectionError::InvalidCharacter { ch: '-' },
                "'-' is not a letter",
            ),
            (
                RejectionError::LengthExceededMaximum { maximum: 5 },
                "the guess already has 5 letters",
            ),
            (
                RejectionError::EmptyGuess,
                "the guess has no letters to delete",
            ),
            (
                RejectionError::IncompleteGuess {
                    letters: 2,
                    word_length: 5,
                    attempt: 0,
                },
                "the guess has 2 of 5 letters",
            ),
            (
                RejectionError::NotInWordBank {
                    word: "ABCDE".into(),
                    attempt: 0,
                },
                "ABCDE is not in the word bank",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
