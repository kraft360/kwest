# Third-party notices

Kwest is released under the [MIT License](LICENSE). It also contains the third-party work listed
below, which stays under its own terms.

Rust crates that Kwest depends on (see `Cargo.lock`) are not bundled in this repository and are
not listed here.

## SCOWL (Spell Checker Oriented Word Lists)

- **Used in:** `kwest-tui/data/words.txt`, the built-in word list. It is compiled into the
  `kwest` binary.
- **Source:** [SCOWL](http://wordlist.aspell.net) version 2020.12.07.
- **Copyright:** 2000-2018 Kevin Atkinson, and the other authors named in the licence file
  (including Princeton University for WordNet, J Ross Beresford, Benjamin Titze and
  Geoff Kuenning).
- **Licence:** permissive, with conditions that the copyright and permission notices be kept.
  The full text is in [`kwest-tui/data/SCOWL-LICENSE`](kwest-tui/data/SCOWL-LICENSE), SCOWL's
  `Copyright` file, unchanged.
- **Changes:** Kwest uses a subset of the lists: common five-letter words, without inflected
  forms, minus a short exclusion list. See [`kwest-tui/data/README.md`](kwest-tui/data/README.md).

Some of these terms cover redistribution in binary form. Any release of the `kwest` binary, such as
a downloadable archive or a package, must include `SCOWL-LICENSE` alongside this file.
