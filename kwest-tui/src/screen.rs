//! The whole screen: every component, stacked and centered on the terminal.
//!
//! The UI is built from a single primitive, [`Label`]: a fixed-size box with one line of text.
//! [`GridRow`](crate::widgets::GridRow) and [`Grid`](crate::widgets::Grid) lay labels out without
//! knowing anything about the game; they and the label are the [`widgets`](crate::widgets). The
//! components wrap them: [`Title`], [`Board`], [`Notice`], [`Keyboard`] and the [`status_bar`].
//! The screen stacks the components and centers them.
//!
//! The components follow the same pattern without sharing a trait:
//!
//! - Each one's size is fixed when it is built, so layouts are computed once.
//! - `draw(writer, origin)` queues the crossterm commands that draw it. The caller flushes once per
//!   frame.
//! - Components that show game state also have `sync(&game)`, which sets their whole state from
//!   the game, and `apply(&result, &game)`, which updates them for the result of a move given the
//!   game after the move.
//!
//! From top to bottom: title, board, notice bar, keyboard and status bar. Every component is built
//! from a [`Profile`], the structure, and a [`Theme`], the look. How big they are comes from the
//! profile: the screen uses the least cluttered of the [`PROFILES`] that fits the terminal. When a
//! resize calls for another profile, the components are built again from the game, in the same
//! theme, so play carries on where it was. A new game reuses the components, with [`Screen::reset`]
//! syncing them to it. When even the most compact profile doesn't fit, the screen draws a banner
//! asking for a bigger terminal instead, and shows the game again as soon as the terminal is
//! resized to fit.

use std::io::Write;

use kwest_core::{Game, Outcome, RejectionError};

use crate::board::Board;
use crate::errors::WidgetError;
use crate::keyboard::Keyboard;
use crate::notice::Notice;
use crate::profile::{PROFILES, Profile, QWERTY, TITLE};
use crate::status::status_bar;
use crate::theme::Theme;
use crate::title::Title;
use crate::widgets::{Geometry, Label, Point, Size};

/// The components built with one profile. Top to bottom: title, board, notice, keyboard and
/// status bar.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Components {
    /// Title.
    title: Title,
    /// Board of guesses.
    board: Board,
    /// Feedback line.
    notice: Notice,
    /// Keyboard.
    keyboard: Keyboard,
    /// Key hints.
    status: Label,
}

impl Components {
    /// Build the components with the profile and theme, showing the game's state.
    fn new(profile: &Profile, theme: &Theme, game: &Game) -> Result<Self, WidgetError> {
        let keyboard = Keyboard::new(QWERTY, profile, theme, game)?;
        // Text lines are as wide as the widest component, the keyboard, so everything lines up.
        let width = keyboard.size().width;
        Ok(Self {
            title: Title::new(TITLE, profile, theme)?,
            board: Board::new(profile, theme, game)?,
            notice: Notice::new(width, profile.wording, theme, game),
            keyboard,
            status: status_bar(width, profile.wording, theme),
        })
    }

    /// Sizes of the components in top to bottom order.
    fn sizes(&self) -> [Size; 5] {
        [
            self.title.size(),
            self.board.size(),
            self.notice.size(),
            self.keyboard.size(),
            self.status.size(),
        ]
    }

    /// Set the components showing game state from the game.
    fn sync(&mut self, game: &Game) {
        self.board.sync(game);
        self.notice.sync(game);
        self.keyboard.sync(game);
    }

    /// Update the components for the result of a game move.
    fn apply(&mut self, result: &Result<Outcome, RejectionError>, game: &Game) {
        self.board.apply(result, game);
        self.notice.apply(result, game);
        self.keyboard.apply(result, game);
    }

    /// Draw the components at their locations, given in top to bottom order.
    fn draw(&self, writer: &mut dyn Write, locations: &[Point]) -> std::io::Result<()> {
        // A layout gives one location per component, so a missing one is a bug; draw nothing.
        let [title, board, notice, keyboard, status] = locations else {
            return Ok(());
        };
        self.title.draw(writer, *title)?;
        self.board.draw(writer, *board)?;
        self.notice.draw(writer, *notice)?;
        self.keyboard.draw(writer, *keyboard)?;
        self.status.draw(writer, *status)
    }
}

/// The whole terminal screen: the components of the best fitting profile, stacked and centered on
/// the terminal. Top to bottom: title, board, notice, keyboard and status bar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Screen {
    /// Minimum terminal size of each profile in [`PROFILES`], for this game's dimensions. Every
    /// game in a session has the same dimensions, so they are worked out once.
    requirements: [Size; PROFILES.len()],
    /// Index in [`PROFILES`] of the profile the components are built with.
    active: usize,
    /// The look, kept to build the components again with it when the profile changes.
    theme: Theme,
    /// Components built with the active profile.
    components: Components,
    /// Current terminal size.
    terminal_size: Size,
    /// Component locations for the current terminal size, in top to bottom order.
    geometry: Geometry,
}

impl Screen {
    /// Create a screen for the game on a terminal of the given size, in the theme's look, built
    /// with the least cluttered profile that fits.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the components do not fit in `u16::MAX`.
    pub fn new(game: &Game, terminal_size: Size, theme: &Theme) -> Result<Self, WidgetError> {
        let mut requirements = [Size::default(); PROFILES.len()];
        for (requirement, profile) in requirements.iter_mut().zip(&PROFILES) {
            *requirement = profile.required_size(game)?;
        }
        let active = best_profile(&requirements, terminal_size);
        let components = Components::new(&PROFILES[active], theme, game)?;
        let geometry = PROFILES[active].layout(&components.sizes(), Some(terminal_size))?;
        Ok(Self {
            requirements,
            active,
            theme: *theme,
            components,
            terminal_size,
            geometry,
        })
    }

    /// Reset the screen for a new game. Only the state of the cells changes; the profile, sizes
    /// and layout are kept.
    pub fn reset(&mut self, game: &Game) {
        self.components.sync(game);
    }

    /// Minimum terminal size for the profile in use. When no profile fits, that is the most
    /// compact one, so it is the size the terminal must at least reach.
    #[must_use]
    pub fn required_size(&self) -> Size {
        self.requirements[self.active]
    }

    /// Does the current terminal fit all the components?
    #[must_use]
    pub fn fits(&self) -> bool {
        fits(self.required_size(), self.terminal_size)
    }

    /// Lay the screen out again for a new terminal size. If another profile now fits better, the
    /// components are built again with it, from the game, so they show the game where it is.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the components do not fit in `u16::MAX`. The
    /// screen is then left as it was.
    pub fn resize(&mut self, terminal_size: Size, game: &Game) -> Result<(), WidgetError> {
        let active = best_profile(&self.requirements, terminal_size);
        // Within the same profile the components are kept; only their locations change.
        let rebuilt = if active == self.active {
            None
        } else {
            Some(Components::new(&PROFILES[active], &self.theme, game)?)
        };
        let sizes = rebuilt.as_ref().unwrap_or(&self.components).sizes();
        let geometry = PROFILES[active].layout(&sizes, Some(terminal_size))?;
        // Nothing can fail from here on, so the screen changes all at once or not at all.
        if let Some(components) = rebuilt {
            self.components = components;
        }
        self.active = active;
        self.geometry = geometry;
        self.terminal_size = terminal_size;
        Ok(())
    }

    /// Update the components for the result of a game move.
    /// `game` is the state after the move was applied.
    pub fn apply(&mut self, result: &Result<Outcome, RejectionError>, game: &Game) {
        self.components.apply(result, game);
    }

    /// Draw the whole screen. If the terminal is too small, a message asking to enlarge it is
    /// drawn instead. The caller flushes.
    ///
    /// # Errors
    ///
    /// Will return `std::io::Error` on rendering error.
    pub fn draw(&self, writer: &mut dyn Write) -> std::io::Result<()> {
        if !self.fits() {
            return self.draw_too_small(writer);
        }
        self.components.draw(writer, &self.geometry.locations)
    }

    /// Draw a message filling the terminal, asking to enlarge it.
    /// The status bar is hidden too, so the message says how to quit. The essentials come first,
    /// as the label truncates on very narrow terminals.
    fn draw_too_small(&self, writer: &mut dyn Write) -> std::io::Result<()> {
        let required_size = self.required_size();
        let message = format!(
            "Terminal too small: need {}x{}, have {}x{} · Enlarge it, or press Esc to quit",
            required_size.width,
            required_size.height,
            self.terminal_size.width,
            self.terminal_size.height
        );
        // The label fills the terminal, so it also wipes whatever was drawn before.
        Label::new(&message, self.terminal_size)
            .with_foreground(self.theme.error)
            .draw(writer, Point::ZERO)
    }
}

/// Is a terminal of the available size at least as big as the required size?
fn fits(required: Size, available: Size) -> bool {
    available.width >= required.width && available.height >= required.height
}

/// Get the index of the first, least cluttered, profile whose requirement fits the terminal, or of
/// the last, most compact, one if none does.
fn best_profile(requirements: &[Size], terminal_size: Size) -> usize {
    requirements
        .iter()
        .position(|&required| fits(required, terminal_size))
        .unwrap_or(requirements.len().saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    use super::*;
    use kwest_core::WordBank;

    /// Letters per word in these tests. `unwrap` in a `const` runs at compile time.
    const WORD_LENGTH: NonZeroUsize = NonZeroUsize::new(5).unwrap();
    /// Guesses allowed in these tests.
    const MAX_ATTEMPTS: NonZeroUsize = NonZeroUsize::new(6).unwrap();

    /// Create a game whose secret is deterministic: the bank has only one word.
    fn create_game() -> Game {
        let bank = WordBank::from_words(["HELLO"], WORD_LENGTH).unwrap();
        Game::new(MAX_ATTEMPTS, Arc::new(bank), &mut rand::rng())
    }

    /// Draw the screen into a string.
    fn render(screen: &Screen) -> String {
        let mut writer = Vec::new();
        screen.draw(&mut writer).unwrap();
        String::from_utf8(writer).unwrap()
    }

    #[test]
    fn required_size_fits_all_components() {
        let screen = Screen::new(&create_game(), Size::new(100, 40), &Theme::STANDARD).unwrap();
        // Keyboard width; 2 + 18 + 1 + 9 + 1 rows plus 4 gaps.
        assert_eq!(screen.required_size(), Size::new(79, 35));
        assert!(screen.fits());
    }

    #[test]
    fn banner_below_compact_asks_for_compact_size() {
        let screen = Screen::new(&create_game(), Size::new(60, 16), &Theme::STANDARD).unwrap();
        assert!(!screen.fits());
        assert!(render(&screen).contains("Terminal too small: need 59x24, have 60x16"));
    }

    #[test]
    fn resize_updates_fit() {
        let game = create_game();
        let mut screen = Screen::new(&game, Size::new(30, 10), &Theme::STANDARD).unwrap();
        screen.resize(Size::new(100, 40), &game).unwrap();
        assert!(screen.fits());
        assert!(!render(&screen).contains("Terminal too small"));
        screen.resize(Size::new(30, 40), &game).unwrap();
        assert!(!screen.fits());
    }

    #[test]
    fn best_profile_at_79x35_is_full() {
        let screen = Screen::new(&create_game(), Size::new(79, 35), &Theme::STANDARD).unwrap();
        assert_eq!(PROFILES[screen.active], Profile::FULL);
        assert!(screen.fits());
    }

    #[test]
    fn best_profile_at_80x24_is_compact() {
        let screen = Screen::new(&create_game(), Size::new(80, 24), &Theme::STANDARD).unwrap();
        assert_eq!(PROFILES[screen.active], Profile::COMPACT);
        assert_eq!(screen.required_size(), Size::new(59, 24));
        assert!(screen.fits());
    }

    #[test]
    fn required_size_matches_built_screen() {
        let game = create_game();
        for profile in PROFILES {
            let components = Components::new(&profile, &Theme::STANDARD, &game).unwrap();
            let built = profile.layout(&components.sizes(), None).unwrap().size;
            assert_eq!(profile.required_size(&game).unwrap(), built);
        }
    }

    #[test]
    fn resize_across_threshold_rebuilds_and_keeps_game_state() {
        let mut game = create_game();
        // Not the default theme, to check that the rebuilt components keep it.
        let theme = Theme::COLORBLIND;
        let mut screen = Screen::new(&game, Size::new(100, 40), &theme).unwrap();
        // Leave a refused guess on the board, the hardest state to rebuild.
        for ch in "ABCDE".chars() {
            let result = game.append(ch);
            screen.apply(&result, &game);
        }
        let result = game.submit();
        screen.apply(&result, &game);
        let full = screen.components.clone();
        screen.resize(Size::new(80, 24), &game).unwrap();
        assert_eq!(
            screen.components,
            Components::new(&Profile::COMPACT, &theme, &game).unwrap()
        );
        assert!(render(&screen).contains("Not in word list"));
        // Back to the full profile, the components are as they were before the switch.
        screen.resize(Size::new(100, 40), &game).unwrap();
        assert_eq!(screen.components, full);
    }

    #[test]
    fn resize_within_profile_keeps_parts() {
        let mut game = create_game();
        let mut screen = Screen::new(&game, Size::new(100, 40), &Theme::STANDARD).unwrap();
        let result = game.append('A');
        screen.apply(&result, &game);
        let components = screen.components.clone();
        screen.resize(Size::new(90, 36), &game).unwrap();
        assert_eq!(screen.components, components);
        assert_eq!(PROFILES[screen.active], Profile::FULL);
    }

    #[test]
    fn components_are_centered_on_the_terminal() {
        let screen = Screen::new(&create_game(), Size::new(81, 40), &Theme::STANDARD).unwrap();
        // Keyboard (79 wide) is offset by 1; the stack (36 high) is offset by 2.
        assert_eq!(screen.geometry.locations[3].x, 1);
        assert_eq!(screen.geometry.locations[0].y, 2);
    }

    #[test]
    fn apply_updates_the_components() {
        let mut game = create_game();
        let mut screen = Screen::new(&game, Size::new(100, 40), &Theme::STANDARD).unwrap();
        for ch in "ABCDE".chars() {
            let result = game.append(ch);
            screen.apply(&result, &game);
        }
        let result = game.submit();
        screen.apply(&result, &game);
        assert!(render(&screen).contains("Not in word list"));
    }

    #[test]
    fn reset_starts_a_blank_game() {
        let mut game = create_game();
        let fresh = Screen::new(&game, Size::new(100, 40), &Theme::STANDARD).unwrap();
        let mut screen = fresh.clone();
        for ch in "HELLO".chars() {
            let result = game.append(ch);
            screen.apply(&result, &game);
        }
        let result = game.submit();
        screen.apply(&result, &game);
        assert_ne!(screen, fresh);
        screen.reset(&create_game());
        assert_eq!(screen, fresh);
    }
}
