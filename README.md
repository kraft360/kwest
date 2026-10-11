# Kwest

A Wordle-style word game for the terminal, written in Rust.

Guess the hidden five-letter word in six tries. After each guess, the letters are colored to show
how close you are, and the on-screen keyboard keeps track of every letter you have tried. Kwest
runs in any terminal, with no browser or account, and you can play as many games in a row as you
like.

![A won game of Kwest: six guesses on the board, the last one, SCOOP, all green, with the
keyboard below showing which letters were tried](docs/screenshot.png)

## Install

You need Rust 1.88 or later, which you can get with [rustup](https://rustup.rs). From a clone of
this repository:

```sh
cargo install --path kwest-tui
```

This builds the `kwest` binary and puts it in `~/.cargo/bin`, which rustup adds to your `PATH`.
The word list is built into the binary, so it runs from any directory. To try it without
installing, use `cargo run --release -p kwest-tui`.

To remove it again: `cargo uninstall kwest-tui`.

## How to play

Run `kwest` and type a five-letter word, then press Enter. Each letter of your guess changes color:

- **green**: the letter is in the word, in this position;
- **yellow**: the letter is in the word, but in another position;
- **grey**: the letter is not in the word (or not as many times as you used it).

Guesses must be real words from the word list; anything else is rejected with "Not in word list"
and doesn't use up a try. For now one list serves as both the possible answers and the accepted
guesses, so it holds only common words: plurals, past tenses and rarer words are refused even
though they are real. Separate lists, a short one for answers and a long one for guesses, are
planned.

The keyboard shows the best result each letter has had so far; letters that may still be in the
word are bold, and ruled-out ones fade to grey. Solve the word within six guesses to win;
otherwise the word is revealed. Either way, press Space to play again.

| Key       | Action                          |
|-----------|---------------------------------|
| A–Z       | Type a letter                   |
| Backspace | Delete a letter                 |
| Enter     | Submit the guess                |
| Space     | Start a new game, once over     |
| Esc       | Quit (Ctrl+C works too)         |

### Colorblind players

If green and yellow are hard to tell apart, run `kwest --colorblind`. Letters are then shown by
shape as well as color: correct letters are underlined, on the board and the keyboard, and the keys
of letters in the word get heavy borders. You can then play without telling green from yellow.

![A won game of Kwest with --colorblind: TIZZY found on the fifth guess, its correct letters
underlined, and the T, Y, I and Z keys underlined with heavy borders](docs/screenshot-colorblind.png)

### Terminal size

The game needs a terminal of at least 59×24 (it fits a standard 80×24 window) and switches to the
roomier layout shown above from 79×35. If the window is too small, Kwest says how large it needs
to be and waits for you to enlarge it, so you can resize it at any point.

## Custom word lists

The built-in list has 2,114 common five-letter English words. To play with your own, pass a text
file with one word per line:

```sh
kwest my-words.txt
```

Case and surrounding spaces don't matter, and duplicates are removed. Lines that are not exactly
five ASCII letters are skipped, so you can use a general word list as it is. The secret is picked
from the same list, and guesses must be in it. If the file can't be read, or has no five-letter
words, Kwest prints why and exits.

## Development

The repository is a Cargo workspace with two crates:

- `kwest-core`: the game engine (word list, rules, scoring). It knows nothing about how the game is
  shown, so other frontends can reuse it.
- `kwest-tui`: the terminal frontend and the `kwest` binary, drawn directly with
  [crossterm](https://crates.io/crates/crossterm).

```sh
cargo fmt
cargo clippy --all-targets   # clippy::pedantic is on; keep it warning-free
cargo test
cargo doc --no-deps
```

CI runs these checks on every push and pull request. The crates are not published on crates.io.

## Licence and credits

Kwest is released under the [MIT License](LICENSE).

The built-in word list, `kwest-tui/data/words.txt`, is derived from
[SCOWL](http://wordlist.aspell.net) (Spell Checker Oriented Word Lists), copyright Kevin Atkinson
and others. It is distributed under SCOWL's own terms, reproduced in
[`kwest-tui/data/SCOWL-LICENSE`](kwest-tui/data/SCOWL-LICENSE). See
[`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md) for all third-party work in this repository,
and [`kwest-tui/data/README.md`](kwest-tui/data/README.md) for how the list is generated.

Kwest is inspired by Wordle, created by Josh Wardle. It is an independent project, not affiliated
with Wordle or The New York Times.
