//! The status bar: one dim line of key hints at the bottom of the screen.
//!
//! The hints never change, so the status bar is a plain [`Label`] rather than a component with
//! state. It is built here rather than in `main` so that its color comes from the theme, like every
//! other color.

use crate::theme::Theme;
use crate::widgets::{Label, Size};
use crate::wording::Wording;

/// Create the status bar: a single dim line of key hints of the given width, in the wording that
/// fits it, in the theme's hint color. It never changes, so it is a plain label rather than a
/// component of its own.
#[must_use]
pub fn status_bar(width: u16, wording: Wording, theme: &Theme) -> Label {
    Label::new(wording.hints(), Size::new(width, 1)).with_foreground(theme.hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_bar_is_one_dim_row() {
        let label = status_bar(79, Wording::Full, &Theme::STANDARD);
        assert_eq!(label.size(), Size::new(79, 1));
        assert_eq!(label.foreground(), Theme::STANDARD.hint);
        assert_eq!(label.content(), Wording::Full.hints());
    }
}
