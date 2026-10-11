//! The data types the engine hands to frontends: tags, letters, guesses, status and move outcomes.
//!
//! These are plain values, with public fields or read-only accessors. The order of [`Tag`]'s
//! variants is part of its meaning: `Absent < Present < Correct`. So the best tag a letter has
//! received is the largest one, which is the last element of a [`LetterHistory`]'s sorted set of
//! tags.

use std::collections::BTreeSet;

/// Score of one letter of a guess against the secret.
///
/// The variants are declared from worst to best, and the derived `Ord` follows declaration
/// order: `Absent < Present < Correct`. Code relies on this to find a letter's best tag, so don't
/// reorder them.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Tag {
    /// The letter does not appear in the secret, or all its occurrences in the secret are already
    /// accounted for by other letters in the guess.
    Absent,
    /// The letter appears in the secret, but at a different position.
    Present,
    /// The letter appears in the secret at exactly this position.
    Correct,
}

/// One letter of a submitted guess, with its score.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Letter {
    /// The letter, in canonical form.
    pub ch: char,
    /// Its score.
    pub tag: Tag,
}

/// Every tag one letter has received in the submitted guesses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LetterHistory {
    /// The letter, in canonical form.
    pub ch: char,
    /// Distinct tags, in ascending order, so `tags.last()` is the best one. Empty if the letter
    /// has not been guessed.
    pub tags: BTreeSet<Tag>,
}

/// A submitted guess: its letters in order, each with its score.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluatedGuess(Vec<Letter>);

impl EvaluatedGuess {
    /// Create an evaluated guess.
    pub(crate) fn new(letters: Vec<Letter>) -> Self {
        Self(letters)
    }

    /// Get the letters in order.
    #[must_use]
    pub fn letters(&self) -> &[Letter] {
        &self.0
    }

    /// Is every letter `Correct`, meaning the guess is the secret?
    #[must_use]
    pub fn is_correct(&self) -> bool {
        self.letters().iter().all(|l| l.tag == Tag::Correct)
    }
}

/// Whether a game is still being played, and how it ended.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    /// The game is still being played.
    InProgress,
    /// The secret was guessed.
    Won,
    /// The attempts ran out.
    Lost {
        /// The secret, revealed now that the game is over.
        secret: String,
    },
}

/// What a successful move changed.
///
/// Each variant says where the change happened, so a frontend can update just that place without
/// asking the game. `attempt` is which guess, from 0 (a row on a board), and `index` is a
/// position within a guess, from 0 (a column).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// A letter was added to the end of the active guess.
    Appended {
        /// The letter, in canonical form.
        ch: char,
        /// The guess it was added to, from 0.
        attempt: usize,
        /// Its position in the guess, from 0.
        index: usize,
    },
    /// The last letter of the active guess was removed.
    Deleted {
        /// The letter.
        ch: char,
        /// The guess it was removed from, from 0.
        attempt: usize,
        /// The position it was removed from, from 0.
        index: usize,
    },
    /// The active guess was scored and recorded.
    Submitted {
        /// The guess that was scored, from 0.
        attempt: usize,
        /// The scored guess.
        guess: EvaluatedGuess,
    },
}
