//! Terminal frontend for Kwest, drawn directly with `crossterm`.
//!
//! [`Guard`] puts the terminal into raw mode on the alternate screen, and restores it when dropped,
//! even after a panic. Hold it for the whole session; entering a second one fails with a
//! [`GuardError`].
//!
//! [`Screen`] shows a game: build it once with the terminal's [`Size`] and a [`Theme`], pass it
//! the result of every move with [`Screen::apply`], and call [`Screen::draw`] each frame, then
//! flush. It is reused for every game through [`Screen::reset`], and follows terminal resizes
//! through [`Screen::resize`], switching between a full and a compact layout to fit the terminal,
//! even mid-game. Building or resizing it fails with a [`WidgetError`] only if the layout doesn't
//! fit in `u16`.
//!
//! [`Theme::STANDARD`] shows tags by color only, and is the default. [`Theme::COLORBLIND`] adds
//! shape cues, so players don't need to tell green from yellow.

// The modules in layers, each using only those below it. To read the code, start with `screen`,
// then one component (`board` is the fullest), then `widgets::label`, the one drawing primitive.

// Entry points: what the binary uses.
mod guard;
mod screen;

// Components: the parts of the screen, which show the game.
mod board;
mod keyboard;
mod notice;
mod status;
mod title;

// Shape and look: what every component is built from.
mod profile;
mod theme;
mod wording;

// Widgets: labels, grids and layouts, with no knowledge of the game.
mod widgets;

// Errors, used by every layer.
mod errors;

pub use errors::{GuardError, WidgetError};
pub use guard::Guard;
pub use screen::Screen;
pub use theme::Theme;
pub use widgets::Size;
