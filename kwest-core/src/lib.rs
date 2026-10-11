//! Game engine for Kwest, a Wordle-style word game.
//!
//! The engine holds the rules and the game state and knows nothing about how they are shown. A
//! frontend drives it with moves and reads the state back through queries.
//!
//! - [`WordBank`] is the list of words a game accepts; the secret is drawn from it too.
//! - [`Game`] is one game. Each move ([`Game::append`], [`Game::delete`], [`Game::submit`]) returns
//!   an [`Outcome`] describing what changed, or a [`RejectionError`] saying why the move was
//!   refused, in which case the game is unchanged. The one exception: a refused submission is
//!   remembered in [`Game::refusal`] until the guess changes, so a frontend can show it again.
//! - A submitted guess becomes an [`EvaluatedGuess`], with each [`Letter`] tagged [`Tag::Correct`],
//!   [`Tag::Present`] or [`Tag::Absent`].
//!
//! Letters are canonical everywhere: uppercase ASCII. Input is normalized, so `'a'` and `'A'` are
//! the same move.
//!
//! ```
//! use std::num::NonZeroUsize;
//! use std::sync::Arc;
//!
//! use kwest_core::{Game, Status, WordBank};
//!
//! // Word length and attempts can't be zero, so they are `NonZeroUsize`. A `const` is checked
//! // when the program compiles.
//! const WORD_LENGTH: NonZeroUsize = NonZeroUsize::new(5).unwrap();
//! const MAX_ATTEMPTS: NonZeroUsize = NonZeroUsize::new(6).unwrap();
//!
//! // With a single word in the bank, the secret is known.
//! let bank = Arc::new(WordBank::from_words(["crane"], WORD_LENGTH)?);
//! let mut game = Game::new(MAX_ATTEMPTS, bank, &mut rand::rng());
//! for ch in "crane".chars() {
//!     game.append(ch)?;
//! }
//! game.submit()?;
//! assert_eq!(game.status(), &Status::Won);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod bank;
mod canonical;
mod errors;
mod eval;
mod game;
mod model;

pub use bank::WordBank;
pub use errors::{RejectionError, WordBankError};
pub use game::Game;
pub use model::{EvaluatedGuess, Letter, LetterHistory, Outcome, Status, Tag};
