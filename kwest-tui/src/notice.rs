//! The notice bar: one line of feedback between the board and the keyboard.
//!
//! It reports the rejections the player needs to know about ("Not in word list", "Not enough
//! letters") and the end of the game, in its own wording rather than the engine's error text. Minor
//! rejections, such as typing into a full guess or pressing a key that isn't a letter, are silent
//! and keep the current message. Any edit to the guess clears it.
//!
//! `apply` updates the message for each move. `sync` sets it from the game alone, using the game's
//! status and the refusal it remembers, so a notice built mid-game shows what the old one did.

use std::io::Write;

use crossterm::style::Color;
use kwest_core::{Game, Outcome, RejectionError, Status};

use crate::theme::Theme;
use crate::widgets::{Label, Point, Size};
use crate::wording::Wording;

/// Message for a guess that isn't in the word list. Short enough for every profile.
const NOT_IN_WORD_LIST: &str = "Not in word list";
/// Message for a guess with too few letters. Short enough for every profile.
const NOT_ENOUGH_LETTERS: &str = "Not enough letters";

/// Single line of feedback for the user: rejected guesses and the end of the game.
/// The width is fixed at construction; longer messages are truncated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notice {
    /// Message, blank when there is nothing to say.
    label: Label,
    /// Texts of the end-of-game messages, short enough for the width.
    wording: Wording,
    /// Colors of the messages. A copy: `Theme` is small and `Copy`, so keeping one is simpler
    /// than borrowing it, which would tie the notice to the theme's lifetime.
    theme: Theme,
}

impl Notice {
    /// Create a notice of the given width showing the game's state, in the theme's colors.
    #[must_use]
    pub fn new(width: u16, wording: Wording, theme: &Theme, game: &Game) -> Self {
        // Build the structure only; `sync` sets the state.
        let mut notice = Self {
            label: Label::new("", Size::new(width, 1)),
            wording,
            theme: *theme,
        };
        notice.sync(game);
        notice
    }

    /// Set the message from the game alone: the end of the game if it is over, otherwise why the
    /// last guess was refused, if it was.
    pub fn sync(&mut self, game: &Game) {
        self.clear();
        match game.status() {
            Status::InProgress => {
                if let Some(error) = game.refusal() {
                    self.show_refusal(error);
                }
            }
            // Every guess was submitted, so their number is the winning guess.
            _ => self.show_status(game, game.evaluated_guesses().len()),
        }
    }

    /// Update the message for the result of a game move.
    /// `game` is the state after the move was applied.
    pub fn apply(&mut self, result: &Result<Outcome, RejectionError>, game: &Game) {
        match result {
            // Editing the guess dismisses any earlier message.
            Ok(Outcome::Appended { .. } | Outcome::Deleted { .. }) => self.clear(),
            // The winning guess's position comes from the result, like the board's rows;
            // `attempt` counts from 0.
            Ok(Outcome::Submitted { attempt, .. }) => self.show_status(game, attempt + 1),
            Err(error) => self.show_refusal(error),
        }
    }

    /// Show the end of the game, won with the given guess counted from 1, or clear the message
    /// if the game goes on.
    fn show_status(&mut self, game: &Game, attempts: usize) {
        match game.status() {
            Status::InProgress => self.clear(),
            Status::Won => self.show(
                &self.wording.won(attempts, game.max_attempts()),
                self.theme.won,
            ),
            Status::Lost { secret } => self.show(&self.wording.lost(secret), self.theme.lost),
        }
    }

    /// Show why a guess was refused. Minor rejections (empty guess, full guess, non-letter keys,
    /// game over) are silent and keep the current message.
    fn show_refusal(&mut self, error: &RejectionError) {
        match error {
            RejectionError::NotInWordBank { .. } => self.show(NOT_IN_WORD_LIST, self.theme.error),
            RejectionError::IncompleteGuess { .. } => {
                self.show(NOT_ENOUGH_LETTERS, self.theme.error);
            }
            _ => {}
        }
    }

    /// Show a message in the given color.
    fn show(&mut self, message: &str, color: Color) {
        self.label.update_content(message);
        self.label.update_foreground(color);
    }

    /// Clear the message.
    fn clear(&mut self) {
        self.show("", self.theme.letter);
    }

    /// Get the size of the notice.
    #[must_use]
    pub fn size(&self) -> Size {
        self.label.size()
    }

    /// Draw the notice with its top-left corner at the given origin.
    ///
    /// # Errors
    ///
    /// Will return `std::io::Error` on rendering error.
    pub fn draw(&self, writer: &mut dyn Write, origin: Point) -> std::io::Result<()> {
        self.label.draw(writer, origin)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    use super::*;
    use kwest_core::WordBank;

    /// Letters per word in these tests. `unwrap` in a `const` runs at compile time.
    const WORD_LENGTH: NonZeroUsize = NonZeroUsize::new(5).unwrap();

    /// Create a game from the words; with a single word the secret is deterministic.
    fn create_game(words: &[&str], max_attempts: usize) -> Game {
        let bank = WordBank::from_words(words, WORD_LENGTH).unwrap();
        Game::new(
            NonZeroUsize::new(max_attempts).unwrap(),
            Arc::new(bank),
            &mut rand::rng(),
        )
    }

    /// Type a word and submit it, applying every outcome to the notice.
    fn play(notice: &mut Notice, game: &mut Game, word: &str) {
        for ch in word.chars() {
            let result = game.append(ch);
            notice.apply(&result, game);
        }
        let result = game.submit();
        notice.apply(&result, game);
    }

    /// Create a full-width notice for the game.
    fn create_notice(game: &Game) -> Notice {
        Notice::new(79, Wording::Full, &Theme::STANDARD, game)
    }

    #[test]
    fn new_notice_is_blank_and_one_row() {
        let notice = create_notice(&create_game(&["HELLO"], 6));
        assert_eq!(notice.size(), Size::new(79, 1));
        assert_eq!(notice.label.content(), "");
    }

    #[test]
    fn unknown_word_is_reported() {
        let mut game = create_game(&["HELLO"], 6);
        let mut notice = create_notice(&game);
        play(&mut notice, &mut game, "ABCDE");
        assert_eq!(notice.label.content(), "Not in word list");
        assert_eq!(notice.label.foreground(), Theme::STANDARD.error);
    }

    #[test]
    fn short_guess_is_reported() {
        let mut game = create_game(&["HELLO"], 6);
        let mut notice = create_notice(&game);
        play(&mut notice, &mut game, "AB");
        assert_eq!(notice.label.content(), "Not enough letters");
    }

    #[test]
    fn editing_clears_the_message() {
        let mut game = create_game(&["HELLO"], 6);
        let mut notice = create_notice(&game);
        play(&mut notice, &mut game, "AB");
        let result = game.delete();
        notice.apply(&result, &game);
        assert_eq!(notice.label.content(), "");
    }

    #[test]
    fn minor_rejections_keep_the_message() {
        let mut game = create_game(&["HELLO"], 6);
        let mut notice = create_notice(&game);
        play(&mut notice, &mut game, "AB");
        notice.apply(&Err(RejectionError::InvalidCharacter { ch: '1' }), &game);
        assert_eq!(notice.label.content(), "Not enough letters");
    }

    #[test]
    fn win_reports_attempts() {
        let mut game = create_game(&["HELLO"], 6);
        let mut notice = create_notice(&game);
        play(&mut notice, &mut game, "HELLO");
        assert_eq!(
            notice.label.content(),
            "Solved in 1/6! Press Space for a new game"
        );
        assert_eq!(notice.label.foreground(), Theme::STANDARD.won);
    }

    #[test]
    fn win_on_a_later_attempt_counts_every_guess() {
        let words = ["HELLO", "WORLD"];
        let mut game = create_game(&words, 6);
        // The secret is random; probe a copy of the game to find it.
        let mut probe = game.clone();
        for ch in words[0].chars() {
            probe.append(ch).unwrap();
        }
        probe.submit().unwrap();
        let (secret, wrong) = if probe.status() == &Status::Won {
            (words[0], words[1])
        } else {
            (words[1], words[0])
        };
        let mut notice = create_notice(&game);
        play(&mut notice, &mut game, wrong);
        play(&mut notice, &mut game, secret);
        assert_eq!(
            notice.label.content(),
            "Solved in 2/6! Press Space for a new game"
        );
    }

    #[test]
    fn loss_reveals_the_secret() {
        let words = ["HELLO", "WORLD"];
        let mut game = create_game(&words, 1);
        // The secret is random; probe a copy of the game to find it.
        let mut probe = game.clone();
        for ch in words[0].chars() {
            probe.append(ch).unwrap();
        }
        probe.submit().unwrap();
        let (secret, wrong) = if probe.status() == &Status::Won {
            (words[0], words[1])
        } else {
            (words[1], words[0])
        };
        let mut notice = create_notice(&game);
        play(&mut notice, &mut game, wrong);
        assert_eq!(
            notice.label.content(),
            format!("The word was {secret}. Press Space for a new game")
        );
        assert_eq!(notice.label.foreground(), Theme::STANDARD.lost);
    }

    #[test]
    fn compact_notice_uses_short_wording() {
        let mut game = create_game(&["HELLO"], 6);
        let mut notice = Notice::new(39, Wording::Compact, &Theme::STANDARD, &game);
        play(&mut notice, &mut game, "HELLO");
        assert_eq!(notice.label.content(), "Solved in 1/6! Space: new game");
    }

    #[test]
    fn notice_built_mid_game_equals_notice_played_move_by_move() {
        let words = ["HELLO", "WORLD"];
        // `<` deletes, `!` submits and `1` is a minor rejection.
        for moves in [
            "",
            "AB!",
            "AB!1",
            "AB!<",
            "ABCDE!",
            "ABCDE!1",
            "WORLD!",
            "WORLD!HELLO!",
        ] {
            let mut game = create_game(&words, 2);
            let mut notice = create_notice(&game);
            for key in moves.chars() {
                let result = match key {
                    '<' => game.delete(),
                    '!' => game.submit(),
                    ch => game.append(ch),
                };
                notice.apply(&result, &game);
            }
            assert_eq!(create_notice(&game), notice, "after {moves:?}");
        }
    }
}
