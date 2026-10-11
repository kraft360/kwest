//! Profiles: how the screen is built, from roomy to compact.
//!
//! A profile is the structure of the screen: the size of the board cells, title cells and keys,
//! which sides of them have a border, the spacing, and the wording of the text lines. It holds no
//! colors; those come from the `Theme`, which is the same in every profile. The screen tries the
//! [`PROFILES`] in order and uses the first that fits the terminal, so the list goes from least to
//! most cluttered.
//!
//! A profile knows how big the screen it builds is. It works that out from sizes alone, with the
//! same layouts the components use, so the screen can pick a profile without building any.

use crossterm::style::Color;
use kwest_core::Game;

use crate::errors::WidgetError;
use crate::widgets::{Axis, Border, ColumnLayout, FlowLayout, Geometry, Size};
use crate::wording::Wording;

/// Title text, one cell per letter.
pub const TITLE: &str = "KWEST";

/// QWERTY keyboard layout, one string per row.
pub const QWERTY: &[&str] = &["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];

/// Profiles from least to most compact; the screen uses the first that fits.
pub const PROFILES: [Profile; 2] = [Profile::FULL, Profile::COMPACT];

/// Border color in a profile, where it means nothing: a profile's borders only say which sides a
/// label has. Components color them from the theme when they build the label.
const UNCOLORED: Color = Color::Reset;

/// How the screen is built: sizes, borders, spacing and wording. The look is in `Theme`.
///
/// Borders set which sides a label has; their color is a placeholder that components replace with
/// the theme's. Components only recolor them later, so a label's size never changes once built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Profile {
    /// Size of a board cell, border included.
    pub board_cell: Size,
    /// Border of a board cell.
    pub board_border: Border,
    /// Size of a title cell, border included. As wide as a board cell, so the title lines up with
    /// the board.
    pub title_cell: Size,
    /// Border of a title cell: an underline, so the title doesn't look like a row of guesses.
    pub title_border: Border,
    /// Size of a keyboard key, border included.
    pub key: Size,
    /// Border of a keyboard key.
    pub key_border: Border,
    /// Spacing between the cells of a board or title row.
    pub cell_spacing: u16,
    /// Spacing between keys in a keyboard row.
    pub key_spacing: u16,
    /// Spacing between the rows of the board and of the keyboard.
    pub row_spacing: u16,
    /// Spacing between the screen components.
    pub component_spacing: u16,
    /// Texts of the notice and status lines, short enough for their width.
    pub wording: Wording,
}

impl Profile {
    /// Boxed cells and keys with room around each letter. Needs 79×35 for a 6×5 game.
    pub const FULL: Profile = Profile {
        board_cell: Size::new(7, 3),
        board_border: Border::all(UNCOLORED),
        title_cell: Size::new(7, 2),
        title_border: Border::bottom(UNCOLORED),
        key: Size::new(7, 3),
        key_border: Border::all(UNCOLORED),
        cell_spacing: 1,
        key_spacing: 1,
        row_spacing: 0,
        component_spacing: 1,
        wording: Wording::Full,
    };

    /// One row per cell and key, so the border colors still show the active and invalid rows. Board
    /// cells and keys have a left border only (`│ A`): with both sides, neighboring cells drew
    /// double bars that were hard to read. So the last cell or key in a row has no closing bar.
    /// Needs 59×24 for a 6×5 game.
    pub const COMPACT: Profile = Profile {
        board_cell: Size::new(5, 1),
        board_border: Border::left(UNCOLORED),
        title_cell: Size::new(5, 2),
        title_border: Border::bottom(UNCOLORED),
        key: Size::new(5, 1),
        key_border: Border::left(UNCOLORED),
        cell_spacing: 1,
        key_spacing: 1,
        row_spacing: 1,
        component_spacing: 1,
        wording: Wording::Compact,
    };

    /// Get the smallest terminal that fits the screen built with this profile for the game.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the screen does not fit in `u16::MAX`.
    pub fn required_size(&self, game: &Game) -> Result<Size, WidgetError> {
        Ok(self.layout(&self.component_sizes(game)?, None)?.size)
    }

    /// Stack the screen components, top to bottom, centered on the terminal if a size is given.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the components do not fit in `u16::MAX`.
    pub fn layout(
        &self,
        sizes: &[Size],
        terminal_size: Option<Size>,
    ) -> Result<Geometry, WidgetError> {
        let layout = ColumnLayout::new(self.component_spacing);
        match terminal_size {
            Some(size) => layout.with_size(size).compute(sizes),
            None => layout.compute(sizes),
        }
    }

    /// Get the sizes the screen components will have for the game, top to bottom: title, board,
    /// notice, keyboard and status. They are what the components' own layouts compute, without
    /// building the components.
    fn component_sizes(&self, game: &Game) -> Result<[Size; 5], WidgetError> {
        let title = row_size(self.title_cell, TITLE.chars().count(), self.cell_spacing)?;
        let board_row = row_size(self.board_cell, game.word_length(), self.cell_spacing)?;
        let board = column_size(&vec![board_row; game.max_attempts()], self.row_spacing)?;
        let key_rows = QWERTY
            .iter()
            .map(|letters| row_size(self.key, letters.chars().count(), self.key_spacing))
            .collect::<Result<Vec<_>, _>>()?;
        let keyboard = column_size(&key_rows, self.row_spacing)?;
        // The text lines are one row, as wide as the keyboard, the widest component.
        let line = Size::new(keyboard.width, 1);
        Ok([title, board, line, keyboard, line])
    }
}

/// Get the size of a row of `count` cells of the same size, laid out like a `GridRow`.
fn row_size(cell: Size, count: usize, spacing: u16) -> Result<Size, WidgetError> {
    let geometry = FlowLayout::new(Axis::Horizontal, spacing).compute(&vec![cell; count])?;
    Ok(geometry.size)
}

/// Get the size of rows stacked like a `Grid`.
fn column_size(rows: &[Size], spacing: u16) -> Result<Size, WidgetError> {
    Ok(ColumnLayout::new(spacing).compute(rows)?.size)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    use super::*;
    use kwest_core::WordBank;
    use unicode_width::UnicodeWidthStr;

    /// Letters per word in these tests. `unwrap` in a `const` runs at compile time.
    const WORD_LENGTH: NonZeroUsize = NonZeroUsize::new(5).unwrap();
    /// Guesses allowed in these tests.
    const MAX_ATTEMPTS: NonZeroUsize = NonZeroUsize::new(6).unwrap();

    /// Create a 6×5 game.
    fn create_game() -> Game {
        let bank = WordBank::from_words(["HELLO"], WORD_LENGTH).unwrap();
        Game::new(MAX_ATTEMPTS, Arc::new(bank), &mut rand::rng())
    }

    #[test]
    fn full_requires_79x35() {
        let size = Profile::FULL.required_size(&create_game()).unwrap();
        assert_eq!(size, Size::new(79, 35));
    }

    #[test]
    fn compact_requires_59x24() {
        let size = Profile::COMPACT.required_size(&create_game()).unwrap();
        assert_eq!(size, Size::new(59, 24));
    }

    #[test]
    fn profiles_go_from_least_to_most_compact() {
        let game = create_game();
        let sizes = PROFILES.map(|profile| profile.required_size(&game).unwrap());
        assert!(sizes[0].width > sizes[1].width && sizes[0].height > sizes[1].height);
    }

    #[test]
    fn wording_fits_its_width() {
        let game = create_game();
        for profile in PROFILES {
            let width = usize::from(profile.required_size(&game).unwrap().width);
            let wording = profile.wording;
            // The longest texts each line can show.
            for text in [
                wording.won(6, 6),
                wording.lost("HELLO"),
                wording.hints().into(),
            ] {
                assert!(text.width() <= width, "{text:?} is wider than {width}");
            }
        }
    }
}
