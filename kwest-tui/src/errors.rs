//! Errors returned by the library.
//!
//! - [`WidgetError`]: building widgets and components, and computing layouts. They fail only by
//!   overflowing the `u16` coordinates of the terminal. Drawing returns `std::io::Error` instead,
//!   since it fails only when the writer does.
//! - [`GuardError`]: preparing the terminal. Either another guard already holds it, or the terminal
//!   itself refused.

use thiserror::Error;

/// Errors building widgets and components, or computing layouts.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum WidgetError {
    /// A size or position would not fit in a terminal coordinate.
    #[error("layout size exceeds the maximum of {maximum_size}")]
    LayoutOverflow {
        /// Largest coordinate allowed.
        maximum_size: u16,
    },
}

/// Errors preparing the terminal with a guard.
#[derive(Debug, Error)]
pub enum GuardError {
    /// Another guard is alive. Only one can hold the terminal at a time, since dropping either
    /// would restore it while the other still expects it prepared.
    #[error("a terminal guard is already alive")]
    AlreadyActive,
    /// The terminal could not be prepared.
    #[error("failed to prepare the terminal")]
    Terminal {
        /// What the terminal reported.
        #[from]
        source: std::io::Error,
    },
}
