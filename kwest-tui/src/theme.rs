//! The theme: the look of the whole UI, as a value.
//!
//! A [`Theme`] holds every color, and whether letters also carry shape cues. It is the only place
//! that maps game state to a look: tags, the active row, invalid guesses, notice messages and keys.
//! Components are built from a theme, as they are from a `Profile`, and never name colors
//! themselves. The two are separate: a profile is the structure, which depends on the terminal
//! size, and a theme is the look, which the player picks and which is the same in every profile.
//!
//! The theme is passed in rather than read from a global, so tests can build components with
//! either theme side by side.

use crossterm::style::{Attribute, Attributes, Color};
use kwest_core::Tag;

use crate::widgets::LineWeight;

/// The look of the UI: every color, and whether letters also carry shape cues.
///
/// The fields are visible only inside the crate: other code picks one of the constants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Theme {
    /// Letters that have no score yet: typed letters and untried keys. `Reset` is the terminal's
    /// own text color, so neutral text is readable on light and dark themes alike; a fixed white
    /// nearly vanished on a light background.
    pub(crate) letter: Color,
    /// Default border, used everywhere except the active and invalid rows of the board.
    pub(crate) border: Color,
    /// Border of cells in the row being typed: the terminal's own text color, brighter than the
    /// default border on a dark theme and darker on a light one. A fixed light grey vanished on a
    /// light theme.
    pub(crate) active_border: Color,
    /// Border of the row being typed when its guess is not in the word list.
    pub(crate) invalid_border: Color,
    /// Letters in the word, in the right place.
    pub(crate) correct: Color,
    /// Letters in the word, in another place.
    pub(crate) present: Color,
    /// Letters not in the word. The palette is Wordle's: Absent fades to dark grey rather than
    /// turning red, so it differs from Correct by brightness, not only by hue. Green against red
    /// is the pair red-green colorblind players can't tell apart.
    pub(crate) absent: Color,
    /// Notice text for a refused guess, and the too-small banner.
    pub(crate) error: Color,
    /// Notice text when the game is won.
    pub(crate) won: Color,
    /// Notice text when the game is lost.
    pub(crate) lost: Color,
    /// Status bar hints.
    pub(crate) hint: Color,
    /// Title letters.
    pub(crate) title: Color,
    /// Whether tags also show by shape, so colors aren't needed to play: Correct letters are
    /// underlined, and the keys of letters in the word have heavy borders.
    pub(crate) shape_cues: bool,
}

impl Theme {
    /// The default look: tags shown by color only.
    pub const STANDARD: Self = Self {
        letter: Color::Reset,
        border: Color::DarkGrey,
        active_border: Color::Reset,
        invalid_border: Color::Red,
        correct: Color::Green,
        present: Color::Yellow,
        absent: Color::DarkGrey,
        error: Color::Red,
        won: Color::Green,
        lost: Color::Yellow,
        hint: Color::DarkGrey,
        title: Color::Cyan,
        shape_cues: false,
    };

    /// The standard look plus shape cues, for players who can't tell green from yellow, the pair
    /// red-green colorblind players confuse.
    // `..Self::STANDARD` copies every other field, so the two differ in exactly one place.
    pub const COLORBLIND: Self = Self {
        shape_cues: true,
        ..Self::STANDARD
    };

    /// Get the letter color that shows a tag.
    #[must_use]
    pub(crate) fn tag_color(&self, tag: Tag) -> Color {
        match tag {
            Tag::Correct => self.correct,
            Tag::Present => self.present,
            Tag::Absent => self.absent,
        }
    }

    /// Get the text attributes that show a tag, on the board and the keyboard. With shape cues,
    /// Correct letters are underlined. Present stays plain, and Absent already differs by
    /// brightness, so every tag can be told apart without its hue.
    #[must_use]
    pub(crate) fn tag_attributes(&self, tag: Tag) -> Attributes {
        match tag {
            Tag::Correct if self.shape_cues => Attributes::from(Attribute::Underlined),
            _ => Attributes::none(),
        }
    }

    /// Get the style of a keyboard key from the best tag the letter has received; `None` if the
    /// letter is untried. Like the board, only the letter carries the tag's color; the border
    /// keeps the default color.
    ///
    /// Every letter that may still be in the answer is bold: untried, Present and Correct. Absent
    /// letters, the only ones ruled out, are not, so they recede. Their dark grey alone was too
    /// close to the untried keys' text color. This is in every theme, since it helps every player.
    /// With shape cues, the keys of letters in the word also get heavy borders, which sets
    /// Present keys apart from untried ones without color.
    #[must_use]
    pub(crate) fn key_style(&self, best: Option<Tag>) -> KeyStyle {
        let attributes = best.map_or(Attributes::none(), |tag| self.tag_attributes(tag));
        let found = matches!(best, Some(Tag::Present | Tag::Correct));
        KeyStyle {
            foreground: best.map_or(self.letter, |tag| self.tag_color(tag)),
            attributes: if best == Some(Tag::Absent) {
                attributes
            } else {
                attributes.with(Attribute::Bold)
            },
            border: self.border,
            border_weight: if found && self.shape_cues {
                LineWeight::Heavy
            } else {
                LineWeight::Light
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// How a keyboard key looks. A struct rather than a tuple, so callers name each part instead of
/// remembering the order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyStyle {
    /// Color of the letter.
    pub foreground: Color,
    /// Text attributes of the letter.
    pub attributes: Attributes,
    /// Color of the key's border.
    pub border: Color,
    /// Line weight of the key's border.
    pub border_weight: LineWeight,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAGS: [Option<Tag>; 4] = [
        None,
        Some(Tag::Absent),
        Some(Tag::Present),
        Some(Tag::Correct),
    ];

    #[test]
    fn colorblind_differs_from_standard_only_in_shape_cues() {
        assert_eq!(
            Theme {
                shape_cues: false,
                ..Theme::COLORBLIND
            },
            Theme::STANDARD
        );
        assert_ne!(Theme::COLORBLIND, Theme::STANDARD);
        assert_eq!(Theme::default(), Theme::STANDARD);
    }

    #[test]
    fn key_style_uses_the_best_tag() {
        let theme = Theme::STANDARD;
        let untried = theme.key_style(None);
        assert_eq!(untried.foreground, theme.letter);
        assert_eq!(untried.border, theme.border);
        for tag in [Tag::Absent, Tag::Present, Tag::Correct] {
            let style = theme.key_style(Some(tag));
            assert_eq!(style.foreground, theme.tag_color(tag));
            assert_eq!(style.border, theme.border);
        }
    }

    #[test]
    fn only_absent_keys_are_not_bold_in_every_theme() {
        for theme in [Theme::STANDARD, Theme::COLORBLIND] {
            for best in TAGS {
                let bold = theme.key_style(best).attributes.has(Attribute::Bold);
                assert_eq!(bold, best != Some(Tag::Absent), "{best:?}");
            }
        }
    }

    #[test]
    fn standard_theme_has_no_shape_cues() {
        let theme = Theme::STANDARD;
        for best in TAGS {
            let style = theme.key_style(best);
            assert!(!style.attributes.has(Attribute::Underlined), "{best:?}");
            assert_eq!(style.border_weight, LineWeight::Light, "{best:?}");
        }
        for tag in [Tag::Absent, Tag::Present, Tag::Correct] {
            assert!(theme.tag_attributes(tag).is_empty());
        }
    }

    #[test]
    fn colorblind_theme_underlines_correct_letters_only() {
        let theme = Theme::COLORBLIND;
        assert!(
            theme
                .tag_attributes(Tag::Correct)
                .has(Attribute::Underlined)
        );
        assert!(theme.tag_attributes(Tag::Present).is_empty());
        assert!(theme.tag_attributes(Tag::Absent).is_empty());
        for best in TAGS {
            let underlined = theme.key_style(best).attributes.has(Attribute::Underlined);
            assert_eq!(underlined, best == Some(Tag::Correct), "{best:?}");
        }
    }

    #[test]
    fn colorblind_theme_gives_found_keys_heavy_borders() {
        let theme = Theme::COLORBLIND;
        let weight = |best| theme.key_style(best).border_weight;
        assert_eq!(weight(None), LineWeight::Light);
        assert_eq!(weight(Some(Tag::Absent)), LineWeight::Light);
        assert_eq!(weight(Some(Tag::Present)), LineWeight::Heavy);
        assert_eq!(weight(Some(Tag::Correct)), LineWeight::Heavy);
    }
}
