//! Wording: the texts of the notice and status lines that depend on the profile.
//!
//! The text lines are only as wide as the keyboard, so a compact profile needs shorter texts. Only
//! the texts that would not fit differ; the short error messages fit everywhere and stay in the
//! notice.

/// Set of texts for the notice and status lines.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Wording {
    /// Full sentences, for lines 79 columns wide.
    Full,
    /// Shorter texts, for lines 59 columns wide.
    Compact,
}

impl Wording {
    /// Get the message for a game won with the given guess, counted from 1.
    #[must_use]
    pub fn won(self, attempts: usize, max_attempts: usize) -> String {
        match self {
            Wording::Full => {
                format!("Solved in {attempts}/{max_attempts}! Press Space for a new game")
            }
            Wording::Compact => format!("Solved in {attempts}/{max_attempts}! Space: new game"),
        }
    }

    /// Get the message for a lost game, revealing the secret.
    #[must_use]
    pub fn lost(self, secret: &str) -> String {
        match self {
            Wording::Full => format!("The word was {secret}. Press Space for a new game"),
            Wording::Compact => format!("It was {secret}. Space: new game"),
        }
    }

    /// Get the key hints of the status bar.
    #[must_use]
    pub fn hints(self) -> &'static str {
        match self {
            Wording::Full => {
                "Enter submit · Backspace delete · Space new game (when over) · Esc quit"
            }
            // The notice says how to start a new game once one is over, so the hint can go.
            Wording::Compact => "Enter submit · Bksp delete · Esc quit",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn won_counts_the_winning_guess() {
        assert_eq!(
            Wording::Full.won(3, 6),
            "Solved in 3/6! Press Space for a new game"
        );
        assert_eq!(Wording::Compact.won(3, 6), "Solved in 3/6! Space: new game");
    }

    #[test]
    fn lost_reveals_the_secret() {
        assert_eq!(
            Wording::Full.lost("CRANE"),
            "The word was CRANE. Press Space for a new game"
        );
        assert_eq!(
            Wording::Compact.lost("CRANE"),
            "It was CRANE. Space: new game"
        );
    }
}
