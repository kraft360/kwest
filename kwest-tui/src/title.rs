//! The title: the game's name, one underlined cell per letter.
//!
//! The cells are as wide as the board's and spaced the same way, so the title lines up with the
//! board, but the profile gives them only a bottom border. A full box would make the title look
//! like another row of guesses. The letters are all in the theme's title color.

use std::io::Write;

use crate::errors::WidgetError;
use crate::profile::Profile;
use crate::theme::Theme;
use crate::widgets::{GridRow, Label, Point, Size};

/// Title shown as a row of underlined letter cells, lined up with the board.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Title {
    /// One cell per letter.
    row: GridRow,
}

impl Title {
    /// Create a title with one cell per character of the text, sized and bordered by the profile
    /// and colored by the theme.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the title is wider than `u16::MAX`.
    pub fn new(text: &str, profile: &Profile, theme: &Theme) -> Result<Self, WidgetError> {
        let labels = text
            .chars()
            .map(|ch| {
                Label::new(&ch.to_string(), profile.title_cell)
                    .with_foreground(theme.title)
                    .with_border(profile.title_border.with_color(theme.border))
            })
            .collect();
        Ok(Self {
            row: GridRow::new(labels, profile.cell_spacing)?,
        })
    }

    /// Get the size of the title.
    #[must_use]
    pub fn size(&self) -> Size {
        self.row.size()
    }

    /// Draw the title with its top-left corner at the given origin.
    ///
    /// # Errors
    ///
    /// Will return `std::io::Error` on rendering error.
    pub fn draw(&self, writer: &mut dyn Write, origin: Point) -> std::io::Result<()> {
        self.row.draw(writer, origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_has_a_cell_per_letter() {
        let title = Title::new("KWEST", &Profile::FULL, &Theme::STANDARD).unwrap();
        assert_eq!(title.row.len(), 5);
        // 5 cells of 7 plus 4 gaps of 1: as wide as a board row. A letter row and an underline.
        assert_eq!(title.size(), Size::new(39, 2));
    }

    #[test]
    fn compact_title_is_as_wide_as_a_compact_board_row() {
        let title = Title::new("KWEST", &Profile::COMPACT, &Theme::STANDARD).unwrap();
        assert_eq!(title.size(), Size::new(29, 2));
    }

    #[test]
    fn title_cells_are_only_underlined() {
        let title = Title::new("AB", &Profile::FULL, &Theme::STANDARD).unwrap();
        for i in 0..2 {
            let border = title.row.label(i).unwrap().border();
            assert_eq!(border.bottom, Some(Theme::STANDARD.border));
            assert_eq!((border.left, border.top, border.right), (None, None, None));
        }
    }

    #[test]
    fn title_letter_sits_on_its_underline() {
        let title = Title::new("A", &Profile::FULL, &Theme::STANDARD).unwrap();
        let mut writer = Vec::new();
        title.draw(&mut writer, Point::ZERO).unwrap();
        let output = String::from_utf8(writer).unwrap();
        // The letter on the first row, then the underline straight below it; no corners.
        let letter = output.find('A').unwrap();
        let underline = output.find('─').unwrap();
        assert!(letter < underline);
        assert!(output.contains(&"─".repeat(7)));
        assert!(!output.contains('└'));
    }

    #[test]
    fn title_letters_have_the_title_color() {
        let title = Title::new("ABCD", &Profile::FULL, &Theme::STANDARD).unwrap();
        for i in 0..4 {
            assert_eq!(
                title.row.label(i).unwrap().foreground(),
                Theme::STANDARD.title
            );
        }
    }
}
