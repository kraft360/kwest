"""Build `words.txt`, the game's word list, from SCOWL.

SCOWL (Spell Checker Oriented Word Lists, http://wordlist.aspell.net) sorts English words into
sizes by how common they are, from 10 (most common) to 95 (rarest). This script keeps the
five-letter words up to `MAX_SIZE`, then removes:

- inflected forms (plurals, third person, past tense, -ing) of words SCOWL knows, since the
  answer should be a word in its own right rather than a form of one;
- the words in `EXCLUDED`, picked by hand: forms the rules above miss, awkward comparatives,
  and words that are vulgar, offensive or painful to land on as a puzzle answer.

Usage: python3 build_words.py <path to unpacked scowl-2020.12.07> > words.txt
"""

USAGE = "usage: python3 build_words.py <path to unpacked scowl-2020.12.07> > words.txt"

import re
import sys
from pathlib import Path

# SCOWL sizes, from most to least common.
SIZES = [10, 20, 35, 40, 50, 55, 60, 70]
# Largest size that answers come from. 40 gives about 2,100 everyday words.
MAX_SIZE = 40

# Words removed by hand, by reason.
EXCLUDED = {
    # Inflected forms the suffix rules miss.
    "elves", "oases", "maxed", "maxes", "pends", "stats", "feted",
    # Comparatives that are rarely used or read oddly as answers.
    "abler", "apter", "barer", "baser", "bluer", "coyer", "cuter", "direr", "drier", "finer",
    "freer", "gayer", "haler", "huger", "icier", "laxer", "lamer", "muter", "newer", "nicer",
    "nuder", "odder", "paler", "purer", "rarer", "rawer", "rifer", "riper", "ruder", "safer",
    "sager", "saner", "shyer", "slier", "sorer", "surer", "tamer", "truer", "viler", "weest",
    "wider", "wiser", "wryer",
    # Rare agent nouns and technical words.
    "dimer", "edger", "liker", "sizer",
    # Vulgar, sexual or offensive words, and words the NYT also removed from Wordle.
    "bimbo", "bitch", "fagot", "fanny", "feces", "horny", "kinky", "lynch", "penis", "prick",
    "pubic", "pussy", "queer", "semen", "slave", "sperm", "spunk", "whore",
}


def load(scowl: Path, sizes: list[int]) -> set[str]:
    """Read the ordinary words (no names or abbreviations) of the given sizes."""
    words: set[str] = set()
    for name in ("english-words", "american-words"):
        for size in sizes:
            path = scowl / "final" / f"{name}.{size}"
            if path.exists():
                # SCOWL files are Latin-1. Non-ASCII words are filtered out later anyway.
                words |= set(path.read_text(encoding="latin-1").split())
    return words


def is_inflected(word: str, vocabulary: set[str]) -> bool:
    """Is the word a plural, third person, past tense or -ing form of a word in `vocabulary`?"""

    def known(stem: str) -> bool:
        return len(stem) >= 2 and stem in vocabulary

    # "-ss" words such as "glass" and "chess" are not plurals.
    if word.endswith("s") and not word.endswith("ss"):
        if known(word[:-1]):  # cats
            return True
        if word.endswith("es") and known(word[:-2]):  # boxes
            return True
        if word.endswith("ies") and known(word[:-3] + "y"):  # flies
            return True
    if word.endswith("ed"):
        if known(word[:-2]) or known(word[:-1]):  # jumped, baked
            return True
        if word.endswith("ied") and known(word[:-3] + "y"):  # cried
            return True
        if word[-3] == word[-4] and known(word[:-3]):  # tipped
            return True
    # doing, icing
    return word.endswith("ing") and (known(word[:-3]) or known(word[:-3] + "e"))


def main() -> None:
    if len(sys.argv) != 2:
        # Status 2 for a usage error, as the `kwest` binary does.
        print(USAGE, file=sys.stderr)
        sys.exit(2)
    scowl = Path(sys.argv[1])
    # Without this check, a wrong path reads no files and prints an empty list.
    if not (scowl / "final").is_dir():
        sys.exit(f"{scowl}: not an unpacked SCOWL release (no final/ directory)\n{USAGE}")
    common = load(scowl, [size for size in SIZES if size <= MAX_SIZE])
    # Stems are looked up in every size, so that a common plural of a rarer word is still caught.
    vocabulary = load(scowl, SIZES)
    words = sorted(
        word
        for word in common
        if re.fullmatch(r"[a-z]{5}", word)
        and not is_inflected(word, vocabulary)
        and word not in EXCLUDED
    )
    if not words:
        sys.exit(f"{scowl}: no five-letter words found")
    print("\n".join(words))


if __name__ == "__main__":
    main()
