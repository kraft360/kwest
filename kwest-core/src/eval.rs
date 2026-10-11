//! Scoring a guess against the secret, with repeated letters handled the way Wordle does.
//!
//! Scoring takes two passes. The first marks the letters in the right position [`Tag::Correct`] and
//! counts the secret's letters that are left unmatched. The second goes left to right and marks a
//! letter [`Tag::Present`] while the secret still has an unmatched copy of it; every other letter
//! is [`Tag::Absent`].
//!
//! For example, `BOBBY` against the secret `ABBEY` scores present, absent, correct, absent,
//! correct. The secret has two Bs: one is matched in place, so only the first of the guess's other
//! two Bs is present.

use std::collections::HashMap;

use crate::model::{EvaluatedGuess, Letter, Tag};

/// Score each letter of the guess against the secret. Both must be in canonical form and have the
/// same number of letters.
///
/// Scoring can't fail, so it returns no `Result`. Checking the guess is `Game`'s job: it refuses an
/// incomplete guess before scoring, and only then does it change anything. If scoring checked too,
/// `Game` would hold an error it could never get, and a `?` that could never fire.
pub(crate) fn evaluate(guess: &str, secret: &str) -> EvaluatedGuess {
    // A mismatch is a bug in the caller. `zip` would quietly score only the shorter length, so
    // catch it in debug builds and tests.
    debug_assert_eq!(
        guess.chars().count(),
        secret.chars().count(),
        "guess and secret must have the same length"
    );
    // First pass: letters in the right position are Correct, and the rest are Absent for now.
    // Count the secret's letters that were not matched in place.
    let mut unmatched_counts: HashMap<char, usize> = HashMap::new();
    let mut letters = Vec::new();
    for (g, s) in guess.chars().zip(secret.chars()) {
        let tag = if g == s { Tag::Correct } else { Tag::Absent };
        letters.push(Letter { ch: g, tag });
        if g != s {
            *unmatched_counts.entry(s).or_default() += 1;
        }
    }
    // Second pass: left to right, a letter that is not Correct becomes Present while the secret
    // still has an unmatched copy of it.
    for letter in &mut letters {
        if letter.tag == Tag::Correct {
            continue;
        }
        if let Some(v) = unmatched_counts.get_mut(&letter.ch)
            && *v > 0
        {
            letter.tag = Tag::Present;
            *v -= 1;
        }
    }
    EvaluatedGuess::new(letters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Tag::{Absent, Correct, Present};

    /// Extract the characters from the guess.
    fn chars(guess: &EvaluatedGuess) -> Vec<char> {
        guess.letters().iter().map(|l| l.ch).collect()
    }

    /// Extract the tags from the guess.
    fn tags(guess: &EvaluatedGuess) -> Vec<Tag> {
        guess.letters().iter().map(|l| l.tag).collect()
    }

    #[test]
    fn extra_copies_of_a_correct_letter_are_marked_absent() {
        let guess = evaluate("EERIE", "SPOKE");
        assert_eq!(chars(&guess), vec!['E', 'E', 'R', 'I', 'E']);
        assert_eq!(tags(&guess), vec![Absent, Absent, Absent, Absent, Correct]);
        assert!(!guess.is_correct());
    }

    #[test]
    fn complete_match_is_marked_all_correct() {
        let guess = evaluate("HELLO", "HELLO");
        assert_eq!(chars(&guess), vec!['H', 'E', 'L', 'L', 'O']);
        assert_eq!(tags(&guess), vec![Correct; 5]);
        assert!(guess.is_correct());
    }

    #[test]
    fn complete_mismatch_is_marked_all_absent() {
        let guess = evaluate("HELLO", "WITTY");
        assert_eq!(chars(&guess), vec!['H', 'E', 'L', 'L', 'O']);
        assert_eq!(tags(&guess), vec![Absent; 5]);
        assert!(!guess.is_correct());
    }

    #[test]
    fn wrongly_located_letters_are_marked_present() {
        let guess = evaluate("TRAIL", "SLIME");
        assert_eq!(chars(&guess), vec!['T', 'R', 'A', 'I', 'L']);
        assert_eq!(tags(&guess), vec![Absent, Absent, Absent, Present, Present]);
        assert!(!guess.is_correct());
    }

    #[test]
    fn repeated_letter_is_tagged_left_to_right() {
        let guess = evaluate("GEESE", "SPREE");
        assert_eq!(chars(&guess), vec!['G', 'E', 'E', 'S', 'E']);
        assert_eq!(
            tags(&guess),
            vec![Absent, Present, Absent, Present, Correct],
        );
        assert!(!guess.is_correct());
    }

    #[test]
    #[should_panic(expected = "same length")]
    #[cfg(debug_assertions)]
    fn length_mismatch_is_a_bug() {
        let _ = evaluate("HELL", "HELLO");
    }
}
