//! Widgets: the building blocks the components are made of, with no knowledge of the game.
//!
//! [`Label`] is the single drawing primitive, [`GridRow`] and [`Grid`] lay labels out, the layouts
//! work out where each child goes, and the model types ([`Point`], [`Size`], [`Border`],
//! [`Geometry`]) describe it all. Nothing here uses `kwest_core`: the components (board, keyboard,
//! notice, title and status bar) decide what the labels show. Keep it that way, so the widgets stay
//! reusable and can be tested without a game.
//!
//! The modules are private, so this file is the gate for the rest of the crate: it re-exports what
//! the components need. The `pub` here reaches only as far as the crate, because `widgets` itself
//! is private in `lib.rs`.

mod grid;
mod label;
mod layout;
mod model;

pub use grid::{Grid, GridRow};
pub use label::Label;
pub use layout::{Axis, ColumnLayout, FlowLayout};
pub use model::{Border, Geometry, LineWeight, Point, Size};
