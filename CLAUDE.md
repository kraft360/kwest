# Kwest

A Wordle-style word game in Rust: a UI-agnostic game engine (`kwest-core`) plus a hand-rolled
terminal frontend (`kwest-tui`) built directly on `crossterm`.

How it is built, and why, is in `ARCHITECTURE.md`, imported here; keep it current when a change
alters what it describes. Players' docs are in `README.md`.

@ARCHITECTURE.md

## Rules

- Keep changes small and reviewable.
- Don't pull in a TUI framework (no ratatui or similar); that is the point of the project.
- `kwest-core` is stable; change it when correctness needs it. Active work happens in `kwest-tui`.
- `words.txt` is generated, not hand-edited: change `build_words.py` (usually `EXCLUDED`) and
  regenerate. Don't add lists copied from Wordle or other games. SCOWL's notice stays next to the
  list in `SCOWL-LICENSE`, and any third-party data added to the repo gets an entry in
  `THIRD-PARTY-NOTICES.md`.
- Docs diagrams are Mermaid, so they render on GitHub and diff like code.

## Commands

```sh
cargo build
cargo test                        # all crates
cargo test -p kwest-core
cargo clippy --all-targets        # pedantic is on; keep it warning-free
cargo fmt
cargo doc --no-deps               # public docs
cargo doc --no-deps --document-private-items -p kwest-tui --lib   # developer docs, internals too
cargo run -p kwest-tui            # full layout at 79x35 or more, compact down to 59x24
cargo run -p kwest-tui -- FILE    # play with another word list (one word per line)
cargo run -p kwest-tui -- --colorblind   # shape cues as well as colors (Theme::COLORBLIND)
```

Toolchain: edition 2024, resolver 3. `rust-version` is 1.88, the measured minimum (let chains);
develop on stable.

CI (`.github/workflows/`): `ci.yml` runs fmt, clippy, test and doc with warnings as errors on
Linux, macOS and Windows, plus `cargo test` on Rust 1.88 (the `msrv` job; keep it in sync with
`rust-version`). `words.yml` regenerates `words.txt` from SCOWL when `kwest-tui/data/` changes and
fails if it differs. `.gitattributes` checks every file out with LF, so Windows builds the same
bytes. The crates are `publish = false` (set once in `[workspace.package]`).

## Conventions

- Change manifests with cargo (`cargo add -p <crate>`, `cargo remove -p <crate>`), not by hand,
  where a subcommand exists. Shared dependency versions go in `[workspace.dependencies]`.
- Work is done when `cargo fmt`, `cargo clippy --all-targets` (no warnings) and `cargo test` pass.
- The workspace `Cargo.toml` enforces the basics as lints: `unsafe_code = "forbid"`, and
  `missing_docs`, `missing_debug_implementations` and `clippy::pedantic` warn. So every public
  item has a `///` doc, every type implements `Debug`, fallible public functions have an
  `# Errors` section, and getters and builders are `#[must_use]`.
- Errors are `thiserror` enums with struct-like variants and documented fields, propagated with
  `?`; no `.unwrap()` outside tests, except in a `const` initializer
  (`NonZeroUsize::new(6).unwrap()`), which runs at compile time and can't panic at run time.
  See `kwest-core/src/errors.rs` and `kwest-tui/src/errors.rs`. `Display` texts are lowercase,
  with no trailing period.
- Modules are private; each crate re-exports its public API from `lib.rs`, and nothing more. A `pub`
  item in a private module is visible only inside the crate, so `lib.rs` is the one gate out of it.
  `kwest-tui` exports only `Screen`, `Guard`, `Size`, `Theme`, `WidgetError` and `GuardError`; the
  components and widgets are internal. A parent module is a gate too: `kwest-tui/src/widgets.rs`
  re-exports what the rest of the crate uses, so import from `crate::widgets`, not from a file
  inside it. Nothing in `widgets/` uses `kwest-core`; code that knows the game is a component.
  `lib.rs` declares the modules grouped by layer (entry points, components, shape and look,
  widgets); a new module goes in its group there and in `ARCHITECTURE.md`. A module with child files
  is `name.rs` next to a `name/` folder, never `name/mod.rs` (the Rust 2018 layout;
  `clippy::mod_module_files` enforces it). Code used only by tests is `#[cfg(test)]`, not exported
  to hide it from dead-code warnings. Every module starts with a `//!` doc saying what it holds and
  why.
- Code is commented densely, in short sentences ending with a period. Comments say why, not what.
- Unit tests go in a `#[cfg(test)] mod tests` at the bottom of the file, with names that describe
  the behavior (`append_at_maximum_length_is_rejected`).
