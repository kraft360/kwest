//! The command line: what the arguments ask for, the help text, and how errors are printed.
//!
//! Parsing uses the standard library rather than a crate like `clap`: one optional path and three
//! flags don't justify a dependency. Arguments are read as `OsString`s because on Unix a file
//! name can be any bytes, not only valid UTF-8.

use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;

use kwest_tui::Theme;
use thiserror::Error;

/// Command line help, printed by `--help` and after a usage error.
pub const USAGE: &str = "\
Usage: kwest [OPTIONS] [WORDS]

Play Kwest, a Wordle-style word game, in the terminal.

Arguments:
  [WORDS]  Word list to play with: a text file with one word per line, in any case.
           Words that are not five letters long are skipped. Defaults to the built-in list.

Options:
      --colorblind  Show letters by shape as well as color: correct letters underlined, and
                    keys of letters in the word with heavy borders
  -h, --help        Print this help
  -V, --version     Print the version";

/// Version line printed by `--version`. Cargo sets `CARGO_PKG_VERSION` from the manifest at
/// compile time, so it can't drift from the released version.
pub const VERSION: &str = concat!("kwest ", env!("CARGO_PKG_VERSION"));

/// What the command line asks for.
#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    /// Play, with the word list in the given file, or the built-in one if there is none.
    Play {
        /// Word list file.
        word_file: Option<PathBuf>,
        /// The look: `Theme::COLORBLIND` with `--colorblind`, otherwise `Theme::STANDARD`.
        theme: Theme,
    },
    /// Print the help.
    Help,
    /// Print the version.
    Version,
}

/// Arguments that don't make sense. The arguments are kept as lossy UTF-8, for the message only.
#[derive(Debug, Eq, Error, PartialEq)]
pub enum ArgsError {
    /// An argument starting with `-` that is not an option of this program.
    #[error("unknown option {option}")]
    UnknownOption {
        /// The argument as given.
        option: String,
    },
    /// A second word list file.
    #[error("unexpected argument {argument}")]
    UnexpectedArgument {
        /// The first extra argument.
        argument: String,
    },
}

/// Parse the arguments after the program name. Options and the word list can come in any order,
/// and help or version wins over everything else.
///
/// # Errors
///
/// Will return `ArgsError::UnknownOption` for an option this program doesn't have, and
/// `ArgsError::UnexpectedArgument` for a second word list file.
pub fn parse_args(args: impl Iterator<Item = OsString>) -> Result<Command, ArgsError> {
    let mut word_file = None;
    let mut theme = Theme::STANDARD;
    for arg in args {
        if arg == "-h" || arg == "--help" {
            return Ok(Command::Help);
        }
        if arg == "-V" || arg == "--version" {
            return Ok(Command::Version);
        }
        if arg == "--colorblind" {
            theme = Theme::COLORBLIND;
            continue;
        }
        // A file whose name starts with `-` can still be given as `./-name`.
        if arg.to_string_lossy().starts_with('-') {
            return Err(ArgsError::UnknownOption {
                option: arg.to_string_lossy().into_owned(),
            });
        }
        if word_file.is_some() {
            return Err(ArgsError::UnexpectedArgument {
                argument: arg.to_string_lossy().into_owned(),
            });
        }
        word_file = Some(PathBuf::from(arg));
    }
    Ok(Command::Play { word_file, theme })
}

/// Join an error and the chain of errors that caused it into one line, outermost first, such as
/// `words.txt: failed to read word list: No such file or directory (os error 2)`.
#[must_use]
pub fn report(error: &(dyn Error + 'static)) -> String {
    // `successors` keeps calling `source` until it returns `None`. Its closure gets a reference to
    // the current item, which is itself a reference; `|&error|` copies that inner reference out,
    // so the cause it returns is not tied to the closure's short-lived borrow.
    std::iter::successors(Some(error), |&error| error.source())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

#[cfg(test)]
mod tests {
    use std::io;

    use kwest_core::WordBankError;

    use super::*;

    /// Parse arguments given as string slices.
    fn parse(args: &[&str]) -> Result<Command, ArgsError> {
        parse_args(args.iter().map(OsString::from))
    }

    #[test]
    fn no_arguments_play_with_the_built_in_list() {
        assert_eq!(
            parse(&[]),
            Ok(Command::Play {
                word_file: None,
                theme: Theme::STANDARD
            })
        );
    }

    #[test]
    fn a_path_plays_with_that_list() {
        assert_eq!(
            parse(&["my-words.txt"]),
            Ok(Command::Play {
                word_file: Some(PathBuf::from("my-words.txt")),
                theme: Theme::STANDARD
            })
        );
    }

    #[test]
    fn colorblind_option_picks_the_colorblind_theme_anywhere() {
        let play = |word_file: Option<&str>| {
            Ok(Command::Play {
                word_file: word_file.map(PathBuf::from),
                theme: Theme::COLORBLIND,
            })
        };
        assert_eq!(parse(&["--colorblind"]), play(None));
        assert_eq!(parse(&["--colorblind", "a.txt"]), play(Some("a.txt")));
        assert_eq!(parse(&["a.txt", "--colorblind"]), play(Some("a.txt")));
    }

    #[test]
    fn help_options_print_the_help() {
        assert_eq!(parse(&["-h"]), Ok(Command::Help));
        assert_eq!(parse(&["--help"]), Ok(Command::Help));
    }

    #[test]
    fn version_options_print_the_version() {
        assert_eq!(parse(&["-V"]), Ok(Command::Version));
        assert_eq!(parse(&["--version"]), Ok(Command::Version));
        // Help or version wins wherever it is.
        assert_eq!(parse(&["--colorblind", "-V"]), Ok(Command::Version));
    }

    #[test]
    fn unknown_option_is_an_error() {
        let error = parse(&["--easy"]).unwrap_err();
        assert_eq!(
            error,
            ArgsError::UnknownOption {
                option: "--easy".into()
            }
        );
        assert_eq!(error.to_string(), "unknown option --easy");
    }

    #[test]
    fn extra_argument_is_an_error() {
        let error = parse(&["a.txt", "b.txt"]).unwrap_err();
        assert_eq!(
            error,
            ArgsError::UnexpectedArgument {
                argument: "b.txt".into()
            }
        );
        assert_eq!(error.to_string(), "unexpected argument b.txt");
    }

    #[test]
    fn report_joins_the_causes() {
        let error = WordBankError::from(io::Error::new(io::ErrorKind::NotFound, "gone"));
        assert_eq!(report(&error), "failed to read word list: gone");
    }
}
