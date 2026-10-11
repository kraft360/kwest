# Word list

`words.txt` holds the 2,114 five-letter words Kwest uses, one per line. It is generated from
[SCOWL](http://wordlist.aspell.net) (Spell Checker Oriented Word Lists) version 2020.12.07 by
`build_words.py`:

- only common words, up to SCOWL size 40, without names or abbreviations;
- no plurals, past tenses or other inflected forms;
- a short hand-picked exclusion list in the script removes awkward comparatives and vulgar or
  offensive words.

To rebuild it, download and unpack
[scowl-2020.12.07.tar.gz](https://downloads.sourceforge.net/project/wordlist/SCOWL/2020.12.07/scowl-2020.12.07.tar.gz),
then run:

```sh
python3 build_words.py path/to/scowl-2020.12.07 > words.txt
```

Commit the script and the list together: when anything in this directory changes, CI
(`.github/workflows/words.yml`) downloads SCOWL, regenerates the list and fails if it differs from
`words.txt`.

SCOWL is copyright Kevin Atkinson and others, and may be redistributed with its notice.
`SCOWL-LICENSE` is the `Copyright` file from that release, unchanged.

2020.12.07 is the latest SCOWL release that includes the word lists. The newer 2026.02.25
release ships only spell-checker dictionaries built from the new SCOWLv2 database; SCOWL's
author says a full release will follow once that format settles.
