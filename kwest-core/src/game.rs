//! A single game: the secret, the guesses so far, and the moves that change them.
//!
//! The player builds the active guess with [`Game::append`] and [`Game::delete`] and scores it with
//! [`Game::submit`]. Each move is all or nothing: it either succeeds and returns an [`Outcome`]
//! describing the change, or returns a [`RejectionError`] and leaves the game as it was. Once a
//! guess matches the secret or the attempts run out, the game is over and every move is rejected
//! with [`RejectionError::GameOver`].
//!
//! The number of attempts is a [`NonZeroUsize`]: a game with no attempts could never end, so the
//! compiler rejects it rather than a runtime check.
//!
//! The word bank is shared through an `Arc`, so cloning a game doesn't copy the word list. That
//! makes it cheap to try moves on a copy without touching the real game.

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::sync::Arc;

use crate::bank::WordBank;
use crate::canonical;
use crate::errors::RejectionError;
use crate::eval::evaluate;
use crate::model::{EvaluatedGuess, LetterHistory, Outcome, Status, Tag};
use rand::Rng;

/// One game: the secret, the submitted guesses, the guess being typed and the status.
#[derive(Clone, Debug)]
pub struct Game {
    /// Number of guesses allowed.
    max_attempts: NonZeroUsize,
    /// Shared, read-only word bank; reused across games.
    bank: Arc<WordBank>,
    /// Word to guess, in canonical form.
    secret: String,
    /// Submitted guesses, oldest first.
    evaluated_guesses: Vec<EvaluatedGuess>,
    /// Guess being typed, in canonical form.
    active_guess: String,
    /// Whether the game is in progress, won or lost.
    status: Status,
    /// Why the last submission was refused, kept until the guess changes. A frontend can show it
    /// again without remembering the result itself, for example when it redraws from scratch.
    refusal: Option<RejectionError>,
}

impl Game {
    /// Start a game whose secret is picked at random from the bank.
    ///
    /// A game with no attempts could never end, so `max_attempts` can't be zero. Passing a plain
    /// number doesn't compile:
    ///
    /// ```compile_fail,E0308
    /// # use std::{num::NonZeroUsize, sync::Arc};
    /// # use kwest_core::{Game, WordBank};
    /// # const WORD_LENGTH: NonZeroUsize = NonZeroUsize::new(5).unwrap();
    /// # let bank = Arc::new(WordBank::from_words(["crane"], WORD_LENGTH)?);
    /// let game = Game::new(0, bank, &mut rand::rng());
    /// # Ok::<(), kwest_core::WordBankError>(())
    /// ```
    pub fn new(max_attempts: NonZeroUsize, bank: Arc<WordBank>, random: &mut impl Rng) -> Self {
        let secret = bank.random_word(random).into();
        Self::with_secret(max_attempts, bank, secret)
    }

    /// Start a game with a given secret. Every game is built here, so a new field is set in one
    /// place; tests use it directly to pick the secret.
    fn with_secret(max_attempts: NonZeroUsize, bank: Arc<WordBank>, secret: String) -> Self {
        debug_assert!(bank.contains(&secret), "the secret must be in the bank");
        Self {
            max_attempts,
            bank,
            secret,
            evaluated_guesses: Vec::new(),
            active_guess: String::new(),
            status: Status::InProgress,
            refusal: None,
        }
    }

    /// Add a letter to the end of the active guess. Lowercase letters are converted to uppercase.
    ///
    /// # Errors
    ///
    /// Will return `RejectionError::GameOver` if the game is over.
    /// Will return `RejectionError::LengthExceededMaximum` if the active guess is full.
    /// Will return `RejectionError::InvalidCharacter` if the character is not an ASCII letter.
    pub fn append(&mut self, ch: char) -> Result<Outcome, RejectionError> {
        if self.is_over() {
            return Err(RejectionError::GameOver);
        }
        // A full guess takes nothing more, so this is checked before the character itself.
        if self.active_guess.chars().count() == self.bank.word_length() {
            return Err(RejectionError::LengthExceededMaximum {
                maximum: self.bank.word_length(),
            });
        }
        // Only letters are accepted, and they are stored in canonical form.
        if !canonical::is_valid(ch) {
            return Err(RejectionError::InvalidCharacter { ch });
        }
        let ch = canonical::character(ch);
        self.active_guess.push(ch);
        // The guess changed, so an earlier refusal no longer applies.
        self.refusal = None;
        Ok(Outcome::Appended {
            ch,
            attempt: self.evaluated_guesses.len(),
            index: self.active_guess.chars().count() - 1,
        })
    }

    /// Remove the last letter of the active guess.
    ///
    /// # Errors
    ///
    /// Will return `RejectionError::GameOver` if the game is over.
    /// Will return `RejectionError::EmptyGuess` if there is nothing to delete.
    pub fn delete(&mut self) -> Result<Outcome, RejectionError> {
        if self.is_over() {
            return Err(RejectionError::GameOver);
        }
        // `pop` returns `None` when the guess is empty.
        let ch = self.active_guess.pop().ok_or(RejectionError::EmptyGuess)?;
        self.refusal = None;
        Ok(Outcome::Deleted {
            ch,
            attempt: self.evaluated_guesses.len(),
            index: self.active_guess.chars().count(),
        })
    }

    /// Score the active guess, record it, and end the game if it was right or the last attempt.
    ///
    /// # Errors
    ///
    /// Will return `RejectionError::GameOver` if the game is over.
    /// Will return `RejectionError::IncompleteGuess` if the active guess is incomplete.
    /// Will return `RejectionError::NotInWordBank` if the active guess is not in the word bank.
    pub fn submit(&mut self) -> Result<Outcome, RejectionError> {
        // Check everything before changing anything, so a refused guess leaves the game as it was,
        // apart from remembering why it was refused.
        if let Err(error) = self.validate_before_submission() {
            self.refusal = Some(error.clone());
            return Err(error);
        }
        self.refusal = None;
        let guess = evaluate(&self.active_guess, &self.secret);
        // Record the guess and start an empty one.
        self.evaluated_guesses.push(guess.clone());
        self.active_guess.clear();
        if guess.is_correct() {
            self.status = Status::Won;
        } else if self.evaluated_guesses.len() == self.max_attempts.get() {
            self.status = Status::Lost {
                secret: self.secret.clone(),
            };
        }
        Ok(Outcome::Submitted {
            attempt: self.evaluated_guesses.len() - 1,
            guess,
        })
    }

    /// Check that the active guess can be submitted.
    fn validate_before_submission(&self) -> Result<(), RejectionError> {
        if self.is_over() {
            return Err(RejectionError::GameOver);
        }
        // The guess must be complete. Scoring relies on this.
        let letters = self.active_guess.chars().count();
        if letters != self.word_length() {
            return Err(RejectionError::IncompleteGuess {
                letters,
                word_length: self.word_length(),
                attempt: self.evaluated_guesses.len(),
            });
        }
        // Only words from the bank are accepted.
        if !self.bank.contains(&self.active_guess) {
            return Err(RejectionError::NotInWordBank {
                word: self.active_guess.clone(),
                attempt: self.evaluated_guesses.len(),
            });
        }
        Ok(())
    }

    /// Is the game over?
    #[must_use]
    pub fn is_over(&self) -> bool {
        !matches!(self.status, Status::InProgress)
    }

    /// Get every distinct tag the letter has received in the submitted guesses.
    /// Lowercase letters are converted to uppercase first.
    #[must_use]
    pub fn history(&self, ch: char) -> LetterHistory {
        let ch = canonical::character(ch);
        let tags = self
            .evaluated_guesses
            .iter()
            .flat_map(EvaluatedGuess::letters)
            .filter(|l| l.ch == ch)
            .map(|l| l.tag)
            .collect::<BTreeSet<Tag>>();
        LetterHistory { ch, tags }
    }

    /// Get the submitted guesses, oldest first.
    #[must_use]
    pub fn evaluated_guesses(&self) -> &[EvaluatedGuess] {
        &self.evaluated_guesses
    }

    /// Get the guess being typed.
    #[must_use]
    pub fn active_guess(&self) -> &str {
        &self.active_guess
    }

    /// Get the number of letters in the secret and every guess.
    #[must_use]
    pub fn word_length(&self) -> usize {
        self.bank.word_length()
    }

    /// Get the number of guesses allowed. It is never zero; it is returned as a plain `usize`
    /// because callers count and index with it.
    #[must_use]
    pub fn max_attempts(&self) -> usize {
        self.max_attempts.get()
    }

    /// Get the number of guesses left.
    #[must_use]
    pub fn pending_attempts(&self) -> usize {
        self.max_attempts
            .get()
            .saturating_sub(self.evaluated_guesses.len())
    }

    /// Get whether the game is in progress, won or lost.
    #[must_use]
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Get why the last submission was refused, if the guess hasn't changed since.
    /// An accepted move that changes the guess clears it; a refused `append` or `delete` leaves it
    /// as it was, since the guess is unchanged.
    #[must_use]
    pub fn refusal(&self) -> Option<&RejectionError> {
        self.refusal.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Tag::Absent, Tag::Correct, Tag::Present};

    /// Guesses allowed in most tests.
    const MAX_ATTEMPTS: usize = 6;

    /// Create a game whose secret is `words[secret_index]` instead of a random pick.
    fn create_game(words: &[&str], secret_index: usize, max_attempts: usize) -> Game {
        let word_length = NonZeroUsize::new(words[0].len()).unwrap();
        let bank = WordBank::from_words(words, word_length).unwrap();
        Game::with_secret(
            NonZeroUsize::new(max_attempts).unwrap(),
            Arc::new(bank),
            words[secret_index].into(),
        )
    }

    /// Create a game won on the first guess.
    fn create_won_game() -> Game {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        let secret = game.secret.clone();
        for ch in secret.chars() {
            game.append(ch).unwrap();
        }
        game.submit().unwrap();
        game
    }

    /// Type a word into the active guess, expecting every letter to be accepted.
    fn type_a_word(game: &mut Game, word: &str) {
        for ch in word.chars() {
            game.append(ch).unwrap();
        }
    }

    #[test]
    fn new_initializes_state() {
        let game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        assert_eq!(game.max_attempts(), MAX_ATTEMPTS);
        assert_eq!(game.secret, "HELLO");
        assert_eq!(game.evaluated_guesses, Vec::<EvaluatedGuess>::new());
        assert_eq!(game.active_guess, "");
        assert_eq!(game.status, Status::InProgress);
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS);
        assert_eq!(game.refusal(), None);
    }

    #[test]
    fn append_updates_active_guess() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        // Lowercase input is stored in canonical form.
        assert_eq!(
            game.append('a'),
            Ok(Outcome::Appended {
                ch: 'A',
                attempt: 0,
                index: 0
            }),
        );
        assert_eq!(game.active_guess, "A");
        assert_eq!(
            game.append('B'),
            Ok(Outcome::Appended {
                ch: 'B',
                attempt: 0,
                index: 1
            }),
        );
        assert_eq!(game.active_guess, "AB");
        assert_eq!(game.evaluated_guesses, Vec::<EvaluatedGuess>::new());
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS);
    }

    #[test]
    fn non_letter_is_rejected() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        assert_eq!(
            game.append('-'),
            Err(RejectionError::InvalidCharacter { ch: '-' }),
        );
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS);
    }

    #[test]
    fn append_at_maximum_length_is_rejected() {
        let active_guess = "WORLD";
        let mut game = create_game(&["HELLO", "WORLD"], 0, 1);
        // Fill the guess, then try to add a letter and a non-letter.
        type_a_word(&mut game, active_guess);
        assert_eq!(
            game.append('Y'),
            Err(RejectionError::LengthExceededMaximum { maximum: 5 }),
        );
        assert_eq!(
            game.append('-'),
            Err(RejectionError::LengthExceededMaximum { maximum: 5 }),
        );
        assert_eq!(game.active_guess, active_guess);
        assert_eq!(game.pending_attempts(), 1);
    }

    #[test]
    fn append_after_game_over_is_rejected() {
        let mut game = create_won_game();
        assert_eq!(game.append('A'), Err(RejectionError::GameOver));
        assert_eq!(game.append('-'), Err(RejectionError::GameOver));
        assert_eq!(game.active_guess, "");
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS - 1);
    }

    #[test]
    fn delete_on_non_empty_guess_works() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        type_a_word(&mut game, "ABC");
        // Letters are deleted from the end.
        assert_eq!(
            game.delete(),
            Ok(Outcome::Deleted {
                ch: 'C',
                attempt: 0,
                index: 2
            })
        );
        assert_eq!(
            game.delete(),
            Ok(Outcome::Deleted {
                ch: 'B',
                attempt: 0,
                index: 1
            })
        );
        assert_eq!(
            game.delete(),
            Ok(Outcome::Deleted {
                ch: 'A',
                attempt: 0,
                index: 0
            })
        );
        assert_eq!(game.delete(), Err(RejectionError::EmptyGuess));
    }

    #[test]
    fn moves_report_the_attempt_they_belong_to() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        type_a_word(&mut game, "WORLD");
        game.submit().unwrap();
        // The first guess is recorded, so the next moves belong to the second.
        assert_eq!(
            game.append('A'),
            Ok(Outcome::Appended {
                ch: 'A',
                attempt: 1,
                index: 0
            })
        );
        assert_eq!(
            game.delete(),
            Ok(Outcome::Deleted {
                ch: 'A',
                attempt: 1,
                index: 0
            })
        );
        type_a_word(&mut game, "LLLLL");
        assert_eq!(
            game.submit(),
            Err(RejectionError::NotInWordBank {
                word: "LLLLL".into(),
                attempt: 1,
            })
        );
    }

    #[test]
    fn delete_of_empty_guess_is_rejected() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        assert_eq!(game.delete(), Err(RejectionError::EmptyGuess));
        assert_eq!(game.active_guess, "");
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS);
    }

    #[test]
    fn delete_after_game_over_is_rejected() {
        let mut game = create_won_game();
        assert_eq!(game.delete(), Err(RejectionError::GameOver));
        assert_eq!(game.active_guess, "");
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS - 1);
    }

    #[test]
    fn correct_guess_results_in_a_win() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        let secret = game.secret.clone();
        type_a_word(&mut game, &secret);
        let outcome = game.submit().unwrap();
        let Outcome::Submitted { attempt, guess } = outcome else {
            panic!("expected 'Submitted', got {outcome:?}");
        };
        assert_eq!(attempt, 0);
        assert!(guess.is_correct());
        assert_eq!(*game.status(), Status::Won);
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS - 1);
    }

    #[test]
    fn correct_guess_in_the_last_attempt_results_in_a_win() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        // Guess wrong on every attempt but the last.
        for _ in 0..MAX_ATTEMPTS - 1 {
            type_a_word(&mut game, "WORLD");
            game.submit().unwrap();
        }
        // Then guess right.
        let secret = game.secret.clone();
        type_a_word(&mut game, &secret);
        let outcome = game.submit().unwrap();
        let Outcome::Submitted { attempt, guess } = outcome else {
            panic!("expected 'Submitted', got {outcome:?}");
        };
        assert_eq!(attempt, MAX_ATTEMPTS - 1);
        assert!(guess.is_correct());
        assert_eq!(*game.status(), Status::Won);
        assert_eq!(game.pending_attempts(), 0);
    }

    #[test]
    fn bad_guesses_results_in_a_loss() {
        let bad_guess = "WORLD";
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        // Guess wrong on every attempt.
        let mut outcome = None;
        for index in 0..MAX_ATTEMPTS {
            type_a_word(&mut game, bad_guess);
            outcome = Some(game.submit().unwrap());
            // The game goes on until the last attempt.
            if index < MAX_ATTEMPTS - 1 {
                assert_eq!(*game.status(), Status::InProgress);
            }
        }
        let Some(Outcome::Submitted { attempt, guess }) = outcome else {
            panic!("expected 'Submitted', got {outcome:?}");
        };
        assert_eq!(attempt, game.max_attempts() - 1);
        assert!(!guess.is_correct());
        assert_eq!(
            *game.status(),
            Status::Lost {
                secret: game.secret.clone()
            }
        );
        assert_eq!(game.pending_attempts(), 0);
    }

    #[test]
    fn submit_after_game_over_is_rejected() {
        let mut game = create_won_game();
        assert_eq!(game.submit(), Err(RejectionError::GameOver));
        assert_eq!(game.active_guess, "");
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS - 1);
    }

    #[test]
    fn submit_of_incomplete_word_is_rejected() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        let invalid_word = "HE";
        type_a_word(&mut game, invalid_word);
        assert_eq!(
            game.submit(),
            Err(RejectionError::IncompleteGuess {
                letters: 2,
                word_length: 5,
                attempt: 0,
            }),
        );
        // The refused guess is kept, so the player can fix it.
        assert_eq!(game.active_guess, invalid_word);
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS);
    }

    #[test]
    fn submit_of_invalid_word_is_rejected() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        let invalid_word = "LLLLL";
        type_a_word(&mut game, invalid_word);
        assert_eq!(
            game.submit(),
            Err(RejectionError::NotInWordBank {
                word: invalid_word.into(),
                attempt: 0,
            })
        );
        // The refused guess is kept, so the player can fix it.
        assert_eq!(game.active_guess, invalid_word);
        assert_eq!(game.pending_attempts(), MAX_ATTEMPTS);
    }

    #[test]
    fn history_fetches_all_tags() {
        let mut game = create_game(&["SPREE", "GEESE", "SCENE"], 0, MAX_ATTEMPTS);
        // Against SPREE, GEESE scores absent, present, absent, present, correct, and SCENE
        // scores correct, absent, present, absent, correct.
        type_a_word(&mut game, "GEESE");
        game.submit().unwrap();
        type_a_word(&mut game, "SCENE");
        game.submit().unwrap();
        assert_eq!(
            game.history('G'),
            LetterHistory {
                ch: 'G',
                tags: BTreeSet::from([Absent]),
            }
        );
        // Lowercase input finds the uppercase letter's history.
        assert_eq!(
            game.history('e'),
            LetterHistory {
                ch: 'E',
                tags: BTreeSet::from([Absent, Present, Correct]),
            }
        );
        assert_eq!(
            game.history('S'),
            LetterHistory {
                ch: 'S',
                tags: BTreeSet::from([Present, Correct]),
            }
        );
        assert_eq!(
            game.history('C'),
            LetterHistory {
                ch: 'C',
                tags: BTreeSet::from([Absent]),
            }
        );
        assert_eq!(
            game.history('N'),
            LetterHistory {
                ch: 'N',
                tags: BTreeSet::from([Absent]),
            }
        );
    }

    #[test]
    fn history_returns_empty_tags_for_an_unused_character() {
        let mut game = create_game(&["SPREE", "GEESE"], 0, MAX_ATTEMPTS);
        type_a_word(&mut game, "GEESE");
        game.submit().unwrap();
        // Z is in no guess.
        assert_eq!(
            game.history('Z'),
            LetterHistory {
                ch: 'Z',
                tags: BTreeSet::new(),
            }
        );
    }

    #[test]
    fn refused_submission_is_remembered() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        type_a_word(&mut game, "LLLLL");
        let error = game.submit().unwrap_err();
        assert_eq!(game.refusal(), Some(&error));
    }

    #[test]
    fn edit_clears_refusal() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        type_a_word(&mut game, "HE");
        game.submit().unwrap_err();
        game.append('L').unwrap();
        assert_eq!(game.refusal(), None);
        game.submit().unwrap_err();
        game.delete().unwrap();
        assert_eq!(game.refusal(), None);
    }

    #[test]
    fn accepted_submission_clears_refusal() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        type_a_word(&mut game, "WORLD");
        // Submitting twice can't happen without an edit, so make the refusal by hand.
        game.refusal = Some(RejectionError::EmptyGuess);
        game.submit().unwrap();
        assert_eq!(game.refusal(), None);
    }

    #[test]
    fn refused_append_keeps_refusal() {
        let mut game = create_game(&["HELLO", "WORLD"], 0, MAX_ATTEMPTS);
        type_a_word(&mut game, "LLLLL");
        let error = game.submit().unwrap_err();
        // The guess is full, so the letter is refused and the guess is unchanged.
        game.append('A').unwrap_err();
        assert_eq!(game.refusal(), Some(&error));
    }
}
