//! The board of guesses: one row per attempt, one cell per letter.
//!
//! Evaluated rows show each letter in its tag's color, and with the colorblind theme Correct
//! letters are also underlined. Borders show only where the player is: the row being typed has the
//! active border, a guess that isn't in the word list turns that border red until the next edit,
//! and every other row keeps the default border.
//!
//! The board is set from the game in two ways. `sync` sets every cell from the game's state; it
//! runs when the board is built, for a new game, and when the screen switches profiles mid-game.
//! During play, `apply` updates only what a move changed, and every position comes from the move's
//! result: its `attempt` is the row and its `index` the column. Tests check that both ways end with
//! the same board.

use std::io::Write;

use crossterm::style::{Attributes, Color};
use kwest_core::{Game, Outcome, RejectionError};

use crate::errors::WidgetError;
use crate::profile::Profile;
use crate::theme::Theme;
use crate::widgets::{Grid, GridRow, Label, Point, Size};

/// Board of guesses: one row per attempt and one cell per letter.
/// Every cell keeps the border sides the profile gave it; state changes only change colors and
/// attributes, so the size never changes. Borders keep the default color except to highlight the
/// active row or flag an invalid word; evaluated letters are shown by the letter alone (its color,
/// and an underline for Correct if the theme has shape cues), to keep the board calm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Board {
    /// Cells, `max_attempts` rows of `word_length` labels.
    grid: Grid,
    /// The look. A copy, since `Theme` is small and `Copy`.
    theme: Theme,
}

impl Board {
    /// Create a board sized for the game with the profile's cells, showing the game's state in the
    /// theme's look.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the board does not fit in `u16::MAX`.
    pub fn new(profile: &Profile, theme: &Theme, game: &Game) -> Result<Self, WidgetError> {
        // Build the structure only; `sync` sets the state.
        let border = profile.board_border.with_color(theme.border);
        let rows = (0..game.max_attempts())
            .map(|_| {
                let labels = (0..game.word_length())
                    .map(|_| Label::new("", profile.board_cell).with_border(border))
                    .collect();
                GridRow::new(labels, profile.cell_spacing)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut board = Self {
            grid: Grid::new(rows, profile.row_spacing)?,
            theme: *theme,
        };
        board.sync(game);
        Ok(board)
    }

    /// Set every cell from the game's state: evaluated rows in their tags' style, the active row
    /// with its letters, marked invalid if the game refused the guess as not in the word list, and
    /// blank rows below. The board keeps its size, so the game must have the same dimensions as the
    /// one it was created for.
    pub fn sync(&mut self, game: &Game) {
        debug_assert_eq!(self.grid.len(), game.max_attempts(), "attempts changed");
        debug_assert_eq!(
            self.grid.row(0).map_or(0, GridRow::len),
            game.word_length(),
            "word length changed"
        );
        let guesses = game.evaluated_guesses();
        // The row being typed follows the evaluated ones; once the game is over there is none.
        let active = (!game.is_over()).then_some(guesses.len());
        let active_border = match game.refusal() {
            Some(RejectionError::NotInWordBank { .. }) => self.theme.invalid_border,
            _ => self.theme.active_border,
        };
        for (index, row) in self.grid.rows_mut().enumerate() {
            if let Some(guess) = guesses.get(index) {
                for (label, letter) in row.labels_mut().zip(guess.letters()) {
                    label.update(
                        &letter.ch.to_string(),
                        self.theme.tag_color(letter.tag),
                        self.theme.tag_attributes(letter.tag),
                        self.theme.border,
                    );
                }
            } else if active == Some(index) {
                // Cells past the typed letters are blank.
                let mut letters = game.active_guess().chars();
                for label in row.labels_mut() {
                    let content = letters.next().map(String::from).unwrap_or_default();
                    label.update(
                        &content,
                        self.theme.letter,
                        Attributes::none(),
                        active_border,
                    );
                }
            } else {
                for label in row.labels_mut() {
                    label.update("", self.theme.letter, Attributes::none(), self.theme.border);
                }
            }
        }
    }

    /// Update the board for the result of a game move.
    /// `game` is the state after the move was applied.
    pub fn apply(&mut self, result: &Result<Outcome, RejectionError>, game: &Game) {
        match result {
            Ok(Outcome::Appended { ch, attempt, index }) => {
                self.set_cell(*attempt, *index, &ch.to_string());
                // Typing clears any invalid marking.
                self.set_row_border(*attempt, self.theme.active_border);
            }
            Ok(Outcome::Deleted { attempt, index, .. }) => {
                self.set_cell(*attempt, *index, "");
                self.set_row_border(*attempt, self.theme.active_border);
            }
            Ok(Outcome::Submitted { attempt, guess }) => {
                // Only the letters show the tag, by color and attributes; the evaluated row's
                // borders go back to default.
                if let Some(row) = self.grid.row_mut(*attempt) {
                    for (label, letter) in row.labels_mut().zip(guess.letters()) {
                        label.update_content(&letter.ch.to_string());
                        label.update_foreground(self.theme.tag_color(letter.tag));
                        label.update_attributes(self.theme.tag_attributes(letter.tag));
                    }
                }
                self.set_row_border(*attempt, self.theme.border);
                // The next row becomes active, unless that was the last guess.
                if !game.is_over() {
                    self.set_row_border(attempt + 1, self.theme.active_border);
                }
            }
            Err(RejectionError::NotInWordBank { attempt, .. }) => {
                self.set_row_border(*attempt, self.theme.invalid_border);
            }
            // Other rejections leave the board unchanged; they are for the notice bar.
            Err(_) => {}
        }
    }

    /// Set the content of a cell, ignoring positions outside the board.
    fn set_cell(&mut self, row: usize, column: usize, content: &str) {
        if let Some(label) = self.grid.label_mut(row, column) {
            label.update_content(content);
        }
    }

    /// Set the border color of every cell in a row, ignoring rows outside the board.
    fn set_row_border(&mut self, row: usize, color: Color) {
        if let Some(row) = self.grid.row_mut(row) {
            for label in row.labels_mut() {
                label.update_border_color(color);
            }
        }
    }

    /// Get the size of the board.
    #[must_use]
    pub fn size(&self) -> Size {
        self.grid.size()
    }

    /// Draw the board with its top-left corner at the given origin.
    ///
    /// # Errors
    ///
    /// Will return `std::io::Error` on rendering error.
    pub fn draw(&self, writer: &mut dyn Write, origin: Point) -> std::io::Result<()> {
        self.grid.draw(writer, origin)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    use super::*;
    use crate::widgets::Border;
    use crossterm::style::Attribute;
    use kwest_core::{Tag, WordBank};

    /// Letters per word in these tests. `unwrap` in a `const` runs at compile time.
    const WORD_LENGTH: NonZeroUsize = NonZeroUsize::new(5).unwrap();

    /// Max attempts for test.
    const MAX_ATTEMPTS: NonZeroUsize = NonZeroUsize::new(6).unwrap();

    /// Create a game whose secret is deterministic: the bank has only one word.
    fn create_game() -> Game {
        let bank = WordBank::from_words(["HELLO"], WORD_LENGTH).unwrap();
        Game::new(MAX_ATTEMPTS, Arc::new(bank), &mut rand::rng())
    }

    /// Type a word, applying each outcome to the board.
    fn type_word(board: &mut Board, game: &mut Game, word: &str) {
        for ch in word.chars() {
            let result = game.append(ch);
            board.apply(&result, game);
        }
    }

    /// Create a game with two words and play the one that isn't the secret, so the game goes on
    /// at the second row.
    fn create_game_after_a_wrong_guess() -> (Board, Game) {
        let words = ["HELLO", "WORLD"];
        let bank = Arc::new(WordBank::from_words(words, WORD_LENGTH).unwrap());
        let mut game = Game::new(MAX_ATTEMPTS, bank, &mut rand::rng());
        // The secret is random; probe a copy of the game to pick the word that is not it.
        let mut probe = game.clone();
        type_word(
            &mut Board::new(&Profile::FULL, &Theme::STANDARD, &probe).unwrap(),
            &mut probe,
            words[0],
        );
        let wrong = if probe.submit().is_ok() && probe.is_over() {
            words[1]
        } else {
            words[0]
        };
        let mut board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        type_word(&mut board, &mut game, wrong);
        let result = game.submit();
        board.apply(&result, &game);
        (board, game)
    }

    /// Get the label at a position.
    fn cell(board: &Board, row: usize, column: usize) -> &Label {
        board.grid.label(row, column).unwrap()
    }

    #[test]
    fn new_board_has_a_cell_per_letter_and_attempt() {
        let game = create_game();
        let board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        assert_eq!(board.grid.len(), MAX_ATTEMPTS.get());
        assert_eq!(board.grid.row(0).unwrap().len(), 5);
        // 5 cells of 7 plus 4 gaps of 1; 6 rows of 3.
        assert_eq!(board.size(), Size::new(39, 18));
    }

    #[test]
    fn new_board_marks_first_row_active() {
        let game = create_game();
        let board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        assert_eq!(
            cell(&board, 0, 0).border(),
            Border::all(Theme::STANDARD.active_border)
        );
        assert_eq!(
            cell(&board, 1, 0).border(),
            Border::all(Theme::STANDARD.border)
        );
    }

    #[test]
    fn appended_letter_fills_the_active_row() {
        let mut game = create_game();
        let mut board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        type_word(&mut board, &mut game, "ab");
        assert_eq!(cell(&board, 0, 0).content(), "A");
        assert_eq!(cell(&board, 0, 1).content(), "B");
        assert_eq!(cell(&board, 0, 2).content(), "");
    }

    #[test]
    fn deleted_letter_clears_its_cell() {
        let mut game = create_game();
        let mut board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        type_word(&mut board, &mut game, "AB");
        let result = game.delete();
        board.apply(&result, &game);
        assert_eq!(cell(&board, 0, 0).content(), "A");
        assert_eq!(cell(&board, 0, 1).content(), "");
    }

    #[test]
    fn submitted_guess_is_colored_by_tag() {
        let mut game = create_game();
        let mut board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        type_word(&mut board, &mut game, "HELLO");
        let result = game.submit();
        board.apply(&result, &game);
        let green = Theme::STANDARD.tag_color(Tag::Correct);
        for column in 0..5 {
            assert_eq!(cell(&board, 0, column).foreground(), green);
            // The standard theme shows tags by color only.
            assert!(cell(&board, 0, column).attributes().is_empty());
            // Borders stay default; only the letters carry the tag.
            assert_eq!(
                cell(&board, 0, column).border(),
                Border::all(Theme::STANDARD.border)
            );
        }
    }

    #[test]
    fn colorblind_theme_underlines_correct_letters() {
        let mut game = create_game();
        let mut board = Board::new(&Profile::FULL, &Theme::COLORBLIND, &game).unwrap();
        type_word(&mut board, &mut game, "HELLO");
        let result = game.submit();
        board.apply(&result, &game);
        for column in 0..5 {
            let attributes = cell(&board, 0, column).attributes();
            assert!(attributes.has(Attribute::Underlined));
        }
    }

    #[test]
    fn wrong_guess_moves_the_active_row_down() {
        let (board, _) = create_game_after_a_wrong_guess();
        assert_eq!(
            cell(&board, 0, 0).border(),
            Border::all(Theme::STANDARD.border)
        );
        assert_eq!(
            cell(&board, 1, 0).border(),
            Border::all(Theme::STANDARD.active_border)
        );
    }

    #[test]
    fn typing_after_a_wrong_guess_fills_the_next_row() {
        let (mut board, mut game) = create_game_after_a_wrong_guess();
        let first_row = cell(&board, 0, 0).clone();
        type_word(&mut board, &mut game, "AB");
        let result = game.delete();
        board.apply(&result, &game);
        assert_eq!(cell(&board, 1, 0).content(), "A");
        assert_eq!(cell(&board, 1, 1).content(), "");
        // The evaluated row is untouched.
        assert_eq!(cell(&board, 0, 0), &first_row);
    }

    #[test]
    fn word_not_in_bank_on_a_later_row_marks_that_row() {
        let (mut board, mut game) = create_game_after_a_wrong_guess();
        type_word(&mut board, &mut game, "ABCDE");
        let result = game.submit();
        board.apply(&result, &game);
        assert_eq!(
            cell(&board, 1, 0).border(),
            Border::all(Theme::STANDARD.invalid_border)
        );
        assert_eq!(
            cell(&board, 0, 0).border(),
            Border::all(Theme::STANDARD.border)
        );
    }

    #[test]
    fn word_not_in_bank_marks_row_invalid_until_next_edit() {
        let mut game = create_game();
        let mut board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        type_word(&mut board, &mut game, "ABCDE");
        let result = game.submit();
        board.apply(&result, &game);
        assert_eq!(
            cell(&board, 0, 0).border(),
            Border::all(Theme::STANDARD.invalid_border)
        );
        // Deleting a letter clears the marking.
        let result = game.delete();
        board.apply(&result, &game);
        assert_eq!(
            cell(&board, 0, 0).border(),
            Border::all(Theme::STANDARD.active_border)
        );
    }

    #[test]
    fn sync_to_new_game_restores_a_fresh_board() {
        let mut game = create_game();
        let fresh = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        let mut board = fresh.clone();
        // Leave state behind: an invalid row, then a won game.
        type_word(&mut board, &mut game, "ABCDE");
        let result = game.submit();
        board.apply(&result, &game);
        for _ in 0..5 {
            let result = game.delete();
            board.apply(&result, &game);
        }
        type_word(&mut board, &mut game, "HELLO");
        let result = game.submit();
        board.apply(&result, &game);
        assert_ne!(board, fresh);
        board.sync(&create_game());
        assert_eq!(board, fresh);
    }

    /// Play the moves on a game and a board, then check that a board built from the game equals
    /// the one updated move by move.
    fn assert_built_equals_played(game: &mut Game, moves: &str) {
        // The colorblind theme sets the most state: colors and attributes.
        let theme = Theme::COLORBLIND;
        let mut board = Board::new(&Profile::FULL, &theme, game).unwrap();
        for key in moves.chars() {
            let result = match key {
                '<' => game.delete(),
                '!' => game.submit(),
                ch => game.append(ch),
            };
            board.apply(&result, game);
        }
        assert_eq!(
            Board::new(&Profile::FULL, &theme, game).unwrap(),
            board,
            "after {moves:?}"
        );
    }

    #[test]
    fn board_built_mid_game_equals_board_played_move_by_move() {
        // `<` deletes and `!` submits.
        let words = ["HELLO", "WORLD", "WHEEL"];
        for moves in [
            "",
            "WO",
            "WOR<",
            "WORLD!WH",
            "ABCDE!",
            "ABCDE!<",
            "WO!",
            "WORLD!WHEEL!HELLO!",
        ] {
            let bank = WordBank::from_words(words, WORD_LENGTH).unwrap();
            let mut game = Game::new(MAX_ATTEMPTS, Arc::new(bank), &mut rand::rng());
            assert_built_equals_played(&mut game, moves);
        }
    }

    #[test]
    fn board_built_after_lost_game_equals_board_played_move_by_move() {
        let bank = WordBank::from_words(["HELLO", "WORLD"], WORD_LENGTH).unwrap();
        let mut game = Game::new(
            NonZeroUsize::new(2).unwrap(),
            Arc::new(bank),
            &mut rand::rng(),
        );
        // Two different guesses use up both attempts whatever the secret, unless one is right.
        assert_built_equals_played(&mut game, "WORLD!HELLO!");
    }

    #[test]
    fn board_built_after_refusal_marks_the_row_invalid() {
        let mut game = create_game();
        for ch in "ABCDE".chars() {
            game.append(ch).unwrap();
        }
        game.submit().unwrap_err();
        let board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        assert_eq!(cell(&board, 0, 4).content(), "E");
        assert_eq!(
            cell(&board, 0, 0).border(),
            Border::all(Theme::STANDARD.invalid_border)
        );
    }

    #[test]
    fn compact_cells_keep_only_left_borders() {
        let board = Board::new(&Profile::COMPACT, &Theme::STANDARD, &create_game()).unwrap();
        // 5 cells of 5 plus 4 gaps of 1; 6 rows of 1 plus 5 gaps of 1.
        assert_eq!(board.size(), Size::new(29, 11));
        assert_eq!(
            cell(&board, 0, 0).border(),
            Border::left(Theme::STANDARD.active_border)
        );
        assert_eq!(
            cell(&board, 1, 0).border(),
            Border::left(Theme::STANDARD.border)
        );
    }

    #[test]
    fn other_rejections_leave_the_board_unchanged() {
        let game = create_game();
        let mut board = Board::new(&Profile::FULL, &Theme::STANDARD, &game).unwrap();
        let before = board.clone();
        board.apply(&Err(RejectionError::EmptyGuess), &game);
        assert_eq!(board, before);
    }
}
