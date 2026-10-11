//! Kwest in the terminal.
//!
//! `main` reads the command line (see [`cli`]), which picks the word list and the [`Theme`], and
//! loads the word list (see [`words`]). Then `play` runs one game after another on a single
//! [`Screen`], in that theme, until the player quits. In each game,
//! `run` is the event loop: it redraws the whole screen and waits for one event. A key press
//! becomes a game move whose result is applied to the screen, and a resize lays the screen out
//! again.

mod cli;
mod words;

use std::error::Error;
use std::io::{Write, stdout};
use std::num::NonZeroUsize;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use crossterm::QueueableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType};
use kwest_core::Game;
use kwest_tui::{Guard, Screen, Size, Theme};
use rand::rng;

use crate::cli::{Command, USAGE, VERSION};

/// Guesses allowed per game. `unwrap` in a `const` runs at compile time, so it can't panic when
/// the game runs.
const MAX_ATTEMPTS: NonZeroUsize = NonZeroUsize::new(6).unwrap();

/// Read the command line, then play or print the help or the version.
/// `main` returns an `ExitCode` instead of a `Result`, so that errors are printed as readable
/// messages rather than in their `Debug` form.
fn main() -> ExitCode {
    // The first argument is the program itself.
    let command = match cli::parse_args(std::env::args_os().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("kwest: {error}\n\n{USAGE}");
            // 2 is the conventional exit code for a usage error.
            return ExitCode::from(2);
        }
    };
    match command {
        Command::Help => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("{VERSION}");
            ExitCode::SUCCESS
        }
        Command::Play { word_file, theme } => match play(word_file.as_deref(), &theme) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("kwest: {}", cli::report(error.as_ref()));
                ExitCode::FAILURE
            }
        },
    }
}

/// Prepare the terminal and play games in the given theme until the player quits.
///
/// # Errors
///
/// Will return an error if the word list cannot be used, or if the terminal fails.
fn play(word_file: Option<&Path>, theme: &Theme) -> Result<(), Box<dyn Error>> {
    // The bank is built once, before the terminal changes, so that a bad word list is reported
    // on the normal screen. It is shared by every game.
    let bank = Arc::new(words::load_bank(word_file)?);
    let mut random = rng();
    // `Arc::clone` copies the pointer and counts one more owner; the word list itself is not
    // copied. Writing it this way, rather than `bank.clone()`, makes that visible.
    let mut game = Game::new(MAX_ATTEMPTS, Arc::clone(&bank), &mut random);
    // If the terminal is too small, the screen shows a banner until it is enlarged.
    let mut screen = Screen::new(&game, terminal_size()?, theme)?;
    // `_guard` keeps the guard alive until `play` returns. `let _ = ...` would drop it, and
    // restore the terminal, straight away.
    let _guard = Guard::enter()?;
    // Lock stdout once for the whole session, so that each queued command does not lock it again.
    // The lock is reentrant, so the guard can still write to stdout on this thread.
    let mut out = stdout().lock();
    // Play until the player quits, reusing the screen for every game.
    while !run(&mut game, &mut screen, &mut out)? {
        game = Game::new(MAX_ATTEMPTS, Arc::clone(&bank), &mut random);
        screen.reset(&game);
    }
    Ok(())
}

/// Play one game, drawing to `out`. Returns `true` if the player quit, or `false` if they asked
/// for a new game.
fn run(game: &mut Game, screen: &mut Screen, out: &mut impl Write) -> Result<bool, Box<dyn Error>> {
    // Start from a clean screen; afterwards every frame overwrites all of its cells.
    out.queue(Clear(ClearType::All))?;
    loop {
        // Queue the drawing of the whole screen, then flush once.
        screen.draw(out)?;
        out.flush()?;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                // In raw mode Ctrl+C arrives as a key instead of a signal; treat it as quit.
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(true);
                }
                // Other shortcuts are not letters to type.
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    continue;
                }
                // While the too-small banner hides the board, only quitting is allowed.
                if !screen.fits() && key.code != KeyCode::Esc {
                    continue;
                }
                let result = match key.code {
                    // New game only once this one is over, so a stray Space cannot discard it.
                    // Mid-game, Space falls through to `append` and is silently rejected.
                    KeyCode::Char(' ') if game.is_over() => return Ok(false),
                    KeyCode::Char(ch) => game.append(ch),
                    KeyCode::Backspace => game.delete(),
                    KeyCode::Enter => game.submit(),
                    KeyCode::Esc => return Ok(true),
                    _ => continue,
                };
                screen.apply(&result, game);
            }
            Event::Resize(width, height) => {
                // `resize` fails only if the layout overflows `u16::MAX`, and then it leaves the
                // screen as it was. That is no reason to end the game: play goes on with the old
                // layout, and the next resize tries again. A terminal that is merely too small is
                // not an error; the screen shows a banner for it.
                let _ = screen.resize(Size::new(width, height), game);
                // Components move, so their old positions must be cleared.
                out.queue(Clear(ClearType::All))?;
            }
            _ => {}
        }
    }
}

/// Get the current terminal size.
fn terminal_size() -> std::io::Result<Size> {
    let (width, height) = terminal::size()?;
    Ok(Size::new(width, height))
}
