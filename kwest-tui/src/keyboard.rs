//! The on-screen keyboard: shows what the player has learned about each letter.
//!
//! It only displays; input comes from the real keyboard. After each submitted guess, the key of
//! every letter in it takes the style of the best tag that letter has received so far, as in
//! Wordle: the tag's color, bold unless the letter is ruled out, and with the colorblind theme an
//! underline and heavy border too. Keys of letters not guessed yet stay neutral. `sync` styles
//! every key from the game at once, for a new keyboard or a new game; `apply` styles only the keys
//! of a submitted guess.

use std::collections::HashMap;
use std::io::Write;

use kwest_core::{Game, Outcome, RejectionError};

use crate::errors::WidgetError;
use crate::profile::Profile;
use crate::theme::Theme;
use crate::widgets::{Grid, GridRow, Label, Point, Size};

/// Keyboard showing the best tag each letter has received. It is display-only; keys are styled
/// after each submitted guess.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Keyboard {
    /// Keys, one grid row per layout row.
    grid: Grid,
    /// Position (row, column) of each letter's key, built once from the layout. The layout never
    /// changes, so neither does this. It owns its data, so the caller's layout doesn't have to
    /// outlive the keyboard and can be built at runtime.
    positions: HashMap<char, (usize, usize)>,
    /// The look. A copy, since `Theme` is small and `Copy`.
    theme: Theme,
}

impl Keyboard {
    /// Create a keyboard with the given layout, one string of letters per row, with the profile's
    /// keys styled from the game in the theme's look. Each letter should appear once; if one is
    /// repeated, only its last key is styled.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the keyboard does not fit in `u16::MAX`.
    pub fn new(
        layout: &[&str],
        profile: &Profile,
        theme: &Theme,
        game: &Game,
    ) -> Result<Self, WidgetError> {
        // Build the structure only; `sync` sets the state.
        let border = profile.key_border.with_color(theme.border);
        let rows = layout
            .iter()
            .map(|letters| {
                let labels = letters
                    .chars()
                    .map(|ch| Label::new(&ch.to_string(), profile.key).with_border(border))
                    .collect();
                GridRow::new(labels, profile.key_spacing)
            })
            .collect::<Result<Vec<_>, _>>()?;
        // `move` gives the inner closure its own copy of `row`; it runs after the outer closure
        // has returned.
        let positions = layout
            .iter()
            .enumerate()
            .flat_map(|(row, letters)| {
                letters
                    .chars()
                    .enumerate()
                    .map(move |(column, ch)| (ch, (row, column)))
            })
            .collect();
        let mut keyboard = Self {
            grid: Grid::new(rows, profile.row_spacing)?,
            positions,
            theme: *theme,
        };
        keyboard.sync(game);
        Ok(keyboard)
    }

    /// Style every key from the best tag its letter has received in the game; keys of letters not
    /// guessed yet are untried.
    pub fn sync(&mut self, game: &Game) {
        // `positions` is read while `grid` is changed. The borrow checker allows it because they
        // are two different fields; going through `self.key_mut` would borrow all of `self`.
        for (&ch, &(row, column)) in &self.positions {
            if let Some(label) = self.grid.label_mut(row, column) {
                Self::style_key(label, ch, game, &self.theme);
            }
        }
    }

    /// Update the keys for the result of a game move.
    /// `game` is the state after the move was applied; only submitted guesses change the keys.
    pub fn apply(&mut self, result: &Result<Outcome, RejectionError>, game: &Game) {
        let Ok(Outcome::Submitted { guess, .. }) = result else {
            return;
        };
        // `key_mut` borrows all of `self` mutably, so `self.theme` can't be read while a key is
        // held. `Theme` is `Copy`, so take a copy first.
        let theme = self.theme;
        for letter in guess.letters() {
            if let Some(label) = self.key_mut(letter.ch) {
                Self::style_key(label, letter.ch, game, &theme);
            }
        }
    }

    /// Style a letter's key from the best tag the letter has received in the game. An associated
    /// function rather than a method: callers already hold a key borrowed from `self`.
    fn style_key(label: &mut Label, ch: char, game: &Game, theme: &Theme) {
        // Tags are ordered Absent < Present < Correct, so the last one is the best.
        let best = game.history(ch).tags.last().copied();
        let style = theme.key_style(best);
        label.update_foreground(style.foreground);
        label.update_attributes(style.attributes);
        label.update_border_color(style.border);
        label.update_border_weight(style.border_weight);
    }

    /// Get the key of a letter, if the layout has one.
    fn key_mut(&mut self, ch: char) -> Option<&mut Label> {
        let &(row, column) = self.positions.get(&ch)?;
        self.grid.label_mut(row, column)
    }

    /// Get the size of the keyboard.
    #[must_use]
    pub fn size(&self) -> Size {
        self.grid.size()
    }

    /// Draw the keyboard with its top-left corner at the given origin.
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
    use crate::profile::QWERTY;
    use crate::widgets::{Border, LineWeight};
    use crossterm::style::{Attribute, Attributes};
    use kwest_core::{Tag, WordBank};

    /// Letters per word in these tests. `unwrap` in a `const` runs at compile time.
    const WORD_LENGTH: NonZeroUsize = NonZeroUsize::new(5).unwrap();
    /// Guesses allowed in these tests.
    const MAX_ATTEMPTS: NonZeroUsize = NonZeroUsize::new(6).unwrap();

    /// Create a game whose secret is deterministic: the bank has only one word.
    fn create_game() -> Game {
        let bank = WordBank::from_words(["SPREE"], WORD_LENGTH).unwrap();
        Game::new(MAX_ATTEMPTS, Arc::new(bank), &mut rand::rng())
    }

    /// Create a full-size QWERTY keyboard for the game, in the standard theme.
    fn create_keyboard(game: &Game) -> Keyboard {
        create_themed_keyboard(&Theme::STANDARD, game)
    }

    /// Create a full-size QWERTY keyboard for the game, in the given theme.
    fn create_themed_keyboard(theme: &Theme, game: &Game) -> Keyboard {
        Keyboard::new(QWERTY, &Profile::FULL, theme, game).unwrap()
    }

    /// Play SPREE, the secret, applying every move to the keyboard. Every guessed letter ends up
    /// Correct.
    fn play_secret(keyboard: &mut Keyboard, game: &mut Game) {
        for ch in "SPREE".chars() {
            let result = game.append(ch);
            keyboard.apply(&result, game);
        }
        let result = game.submit();
        keyboard.apply(&result, game);
    }

    #[test]
    fn qwerty_keyboard_size() {
        let keyboard = create_keyboard(&create_game());
        // Widest row is 10 keys of 7 plus 9 gaps of 1; 3 rows of 3.
        assert_eq!(keyboard.size(), Size::new(79, 9));
    }

    #[test]
    fn compact_qwerty_keyboard_size() {
        let keyboard =
            Keyboard::new(QWERTY, &Profile::COMPACT, &Theme::STANDARD, &create_game()).unwrap();
        // Widest row is 10 keys of 5 plus 9 gaps of 1; 3 rows of 1 plus 2 gaps of 1.
        assert_eq!(keyboard.size(), Size::new(59, 5));
    }

    #[test]
    fn new_keyboard_keys_are_untried() {
        let mut keyboard = create_keyboard(&create_game());
        let key = keyboard.key_mut('M').unwrap();
        assert_eq!(key.content(), "M");
        assert_eq!(key.foreground(), Theme::STANDARD.letter);
        assert_eq!(key.attributes(), Attributes::from(Attribute::Bold));
        assert_eq!(key.border(), Border::all(Theme::STANDARD.border));
    }

    #[test]
    fn letter_outside_layout_has_no_key() {
        let mut keyboard = create_keyboard(&create_game());
        assert!(keyboard.key_mut('1').is_none());
    }

    #[test]
    fn every_letter_finds_the_key_showing_it() {
        let mut keyboard = create_keyboard(&create_game());
        for ch in QWERTY.iter().flat_map(|letters| letters.chars()) {
            assert_eq!(keyboard.key_mut(ch).unwrap().content(), ch.to_string());
        }
    }

    #[test]
    fn layout_built_at_runtime_can_be_dropped() {
        let rows = vec![String::from("AB"), String::from("C")];
        let layout: Vec<&str> = rows.iter().map(String::as_str).collect();
        let mut keyboard =
            Keyboard::new(&layout, &Profile::FULL, &Theme::STANDARD, &create_game()).unwrap();
        // The keyboard keeps its own copy, so the layout can go away first.
        drop(layout);
        drop(rows);
        assert_eq!(keyboard.key_mut('C').unwrap().content(), "C");
    }

    #[test]
    fn submitted_guess_colors_its_keys() {
        let mut game = create_game();
        let mut keyboard = create_keyboard(&game);
        play_secret(&mut keyboard, &mut game);
        let theme = Theme::STANDARD;
        for ch in "SPRE".chars() {
            let key = keyboard.key_mut(ch).unwrap();
            assert_eq!(key.foreground(), theme.tag_color(Tag::Correct));
            // The standard theme has no shape cues: bold only, light border.
            assert_eq!(key.attributes(), Attributes::from(Attribute::Bold));
            assert_eq!(key.border(), Border::all(theme.border));
        }
        let untouched = keyboard.key_mut('Q').unwrap();
        assert_eq!(untouched.foreground(), theme.letter);
    }

    #[test]
    fn colorblind_theme_marks_found_keys_by_shape() {
        let mut game = create_game();
        let mut keyboard = create_themed_keyboard(&Theme::COLORBLIND, &game);
        play_secret(&mut keyboard, &mut game);
        for ch in "SPRE".chars() {
            let key = keyboard.key_mut(ch).unwrap();
            assert!(key.attributes().has(Attribute::Underlined));
            assert_eq!(key.border().weight, LineWeight::Heavy);
        }
        let untouched = keyboard.key_mut('Q').unwrap();
        assert_eq!(untouched.border().weight, LineWeight::Light);
    }

    #[test]
    fn sync_to_new_game_makes_every_key_untried() {
        let mut game = create_game();
        // The colorblind theme sets the most state, heavy borders included.
        let fresh = create_themed_keyboard(&Theme::COLORBLIND, &game);
        let mut keyboard = fresh.clone();
        play_secret(&mut keyboard, &mut game);
        assert_ne!(keyboard, fresh);
        keyboard.sync(&create_game());
        assert_eq!(keyboard, fresh);
    }

    #[test]
    fn keyboard_built_mid_game_equals_keyboard_played_move_by_move() {
        let bank = WordBank::from_words(["SPREE", "PRESS", "TREES"], WORD_LENGTH).unwrap();
        let mut game = Game::new(MAX_ATTEMPTS, Arc::new(bank), &mut rand::rng());
        let theme = Theme::COLORBLIND;
        let mut keyboard = create_themed_keyboard(&theme, &game);
        for word in ["PRESS", "TREES"] {
            for ch in word.chars() {
                let result = game.append(ch);
                keyboard.apply(&result, &game);
            }
            let result = game.submit();
            keyboard.apply(&result, &game);
            assert_eq!(
                create_themed_keyboard(&theme, &game),
                keyboard,
                "after {word}"
            );
        }
    }

    #[test]
    fn typing_and_rejections_leave_keys_unchanged() {
        let mut game = create_game();
        let mut keyboard = create_keyboard(&game);
        let before = keyboard.clone();
        let result = game.append('S');
        keyboard.apply(&result, &game);
        keyboard.apply(&Err(RejectionError::EmptyGuess), &game);
        assert_eq!(keyboard, before);
    }
}
