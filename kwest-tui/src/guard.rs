//! The terminal guard: puts the terminal into the state the game needs, and restores it afterwards.
//!
//! [`Guard::enter`] switches to raw mode (keys arrive one at a time without being echoed, and
//! Ctrl+C arrives as a key instead of a signal), switches to the alternate screen, and hides the
//! cursor. Dropping the guard undoes all of it. A panic hook does the same, so a crash still leaves
//! a usable terminal, and the panic message is printed on the normal screen where it can be read.
//!
//! The hook is installed once per process and restores the terminal only while a guard is alive. A
//! panic after the guard is gone leaves the terminal alone, because it is already restored. Only
//! one guard can be alive at a time: [`Guard::enter`] refuses a second, since dropping either would
//! restore the terminal while the other still expects it prepared.

use std::io::stdout;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{cursor, execute};

use crate::errors::GuardError;

/// Whether a guard holds the terminal and it is not yet restored. The panic hook can run on any
/// thread, so this is shared state; an atomic is the safe way to share a flag without a lock (a
/// `static mut` would need `unsafe`).
static PREPARED: AtomicBool = AtomicBool::new(false);

/// Makes sure the panic hook is installed only once, even if `enter` is called again.
static HOOK: Once = Once::new();

/// Keeps the terminal prepared for the game while it is alive. Hold it for the whole session:
/// dropping it restores the terminal.
#[derive(Debug)]
pub struct Guard {
    /// Private, so that the only way to get a guard is `enter`, which prepares the terminal.
    _private: (),
}

impl Guard {
    /// Prepare the terminal: raw mode, the alternate screen and a hidden cursor.
    ///
    /// # Errors
    ///
    /// Will return `GuardError::AlreadyActive` if another guard is alive; the terminal is left as
    /// it is. Will return `GuardError::Terminal` if the terminal cannot be prepared. Any step
    /// already taken is undone first.
    pub fn enter() -> Result<Self, GuardError> {
        // Claim the terminal: `compare_exchange` sets the flag only if it is still false, as one
        // step, so two callers can't both succeed.
        if PREPARED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(GuardError::AlreadyActive);
        }
        // Install the hook before changing the terminal, so that a panic from here on restores it.
        HOOK.call_once(install_panic_hook);
        if let Err(error) = enable_raw_mode() {
            // Nothing was changed, so release the claim without restoring anything.
            PREPARED.store(false, Ordering::SeqCst);
            return Err(error.into());
        }
        // From here on, dropping the guard restores the terminal, even if the next step fails.
        let guard = Self { _private: () };
        execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
        Ok(guard)
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        restore();
    }
}

/// Install a panic hook that restores the terminal and then runs the previous hook, so that the
/// panic message is printed on the normal screen.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore();
        original_hook(panic_info);
    }));
}

/// Restore the terminal if it is still prepared. A panic restores it in the hook, and then the
/// guard is dropped while unwinding; `swap` clears the flag and returns its old value in one step,
/// so only the first of the two does the work. Errors are ignored: neither `drop` nor a panic
/// hook can report them, and the remaining steps should run anyway.
fn restore() {
    if PREPARED.swap(false, Ordering::SeqCst) {
        let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_guard_is_refused() {
        // Stand in for a live guard, so the test needs no real terminal.
        PREPARED.store(true, Ordering::SeqCst);
        let result = Guard::enter();
        // Release the claim before asserting, so a failure can't leave it set.
        PREPARED.store(false, Ordering::SeqCst);
        assert!(matches!(result, Err(GuardError::AlreadyActive)));
    }
}
