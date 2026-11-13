//! Tone placement (spec §3.4).
//!
//! Answers "which vowel takes the mark?" and nothing else; applying the mark needs
//! the tone table, which arrives with the tone keys in T1.4.

// Tone placement is wired into the engine in T1.4. Until then nothing outside the
// tests calls it, and an unused item is a warning this project treats as an error.
// Remove this attribute in T1.4 rather than widening it.
#![allow(dead_code)]

use crate::syllable::Syllable;
use crate::unicode::{base_letter, letter_mark};
use crate::ToneStyle;

/// Index of the vowel that carries the tone, relative to the start of the nucleus.
///
/// `None` when the syllable has no nucleus vowel. The rules are ordered because
/// they disagree: `tiếng` is closed (rule 2) *and* carries a quality diacritic
/// (rule 1), and `toán` is closed with two nucleus vowels.
pub(crate) fn tone_index(syllable: &Syllable, style: ToneStyle) -> Option<usize> {
    // 1. A vowel with a quality diacritic wins, and with several the last one takes
    //    the mark: `ướ`, `uyễ`. Anything else would put the mark on a letter that
    //    is already modified.
    if let Some(index) = last_quality_vowel(syllable) {
        return Some(index);
    }

    // 2. Closed syllable: the mark sits on the last nucleus vowel (`toán`).
    if syllable.has_coda() {
        return syllable.nucleus_length.checked_sub(1);
    }

    // 3. Open syllable.
    match syllable.nucleus_length {
        0 => None,
        1 => Some(0),
        // `oa`, `oe` and `uy` are the only open pairs the two conventions disagree
        // about: old `hoà`/`thuỷ`, new `hòa`/`thủy`. `cái` and `của` stay on their
        // first vowel under both, which is why the rule cannot be "second vowel".
        2 => Some(usize::from(
            style == ToneStyle::Old && has_old_style_placement(syllable),
        )),
        // Three vowels: the middle one (`khuỷu`).
        3 => Some(1),
        _ => None,
    }
}

/// Index of the last nucleus vowel carrying a quality diacritic (`ă â ê ô ơ ư`).
fn last_quality_vowel(syllable: &Syllable) -> Option<usize> {
    // A nucleus longer than the array cannot come out of the parser, but a syllable
    // built by hand must not index out of bounds.
    let length = syllable.nucleus_length.min(syllable.nucleus.len());
    (0..length)
        .rev()
        .find(|index| letter_mark(syllable.nucleus[*index]).is_some())
}

/// Whether this open two-vowel nucleus is one the old convention marks on the
/// second vowel.
fn has_old_style_placement(syllable: &Syllable) -> bool {
    let first = base_letter(syllable.nucleus[0]).to_ascii_lowercase();
    let second = base_letter(syllable.nucleus[1]).to_ascii_lowercase();
    matches!((first, second), ('o', 'a') | ('o', 'e') | ('u', 'y'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syllable::parse;

    /// Tone position of `word`, which is the *untoned* spelling of the target:
    /// `toan` stands for `toán`.
    fn position(word: &str, style: ToneStyle) -> Option<usize> {
        tone_index(&parse(word).expect("one syllable"), style)
    }

    fn both_styles(word: &str) -> (Option<usize>, Option<usize>) {
        (
            position(word, ToneStyle::New),
            position(word, ToneStyle::Old),
        )
    }

    #[test]
    fn quality_diacritic_wins_over_position() {
        // `ươ` carries two quality marks and the last one takes the tone: `ướ`.
        assert_eq!(both_styles("ươ"), (Some(1), Some(1)));
        // `uyê` in `uyễ`: the ê is last and marked.
        assert_eq!(both_styles("uyê"), (Some(2), Some(2)));
        // `iê` in `tiếng`: marked vowel, even though the syllable also has a coda.
        assert_eq!(both_styles("tiêng"), (Some(1), Some(1)));
        // `ă` in `hoặc`.
        assert_eq!(both_styles("hoăc"), (Some(1), Some(1)));
    }

    #[test]
    fn closed_syllables_mark_the_last_nucleus_vowel() {
        assert_eq!(both_styles("toan"), (Some(1), Some(1)));
        assert_eq!(both_styles("an"), (Some(0), Some(0)));
        assert_eq!(both_styles("khoan"), (Some(1), Some(1)), "`khoán`");
    }

    #[test]
    fn open_syllables_follow_the_vowel_count() {
        assert_eq!(both_styles("cai"), (Some(0), Some(0)), "`cái`");
        assert_eq!(both_styles("cua"), (Some(0), Some(0)), "`của`");
        assert_eq!(both_styles("a"), (Some(0), Some(0)));
        assert_eq!(both_styles("khuyu"), (Some(1), Some(1)), "`khuỷu`");
        assert_eq!(both_styles("khuya"), (Some(1), Some(1)));
    }

    #[test]
    fn only_oa_oe_uy_differ_between_the_styles() {
        assert_eq!(both_styles("hoa"), (Some(0), Some(1)), "`hòa` vs `hoà`");
        assert_eq!(both_styles("hoe"), (Some(0), Some(1)), "`hòe` vs `hoè`");
        assert_eq!(both_styles("uy"), (Some(0), Some(1)), "`thủy` vs `thuỷ`");
        // Counter-examples: the second vowel is not always the old-style choice.
        assert_eq!(both_styles("ai"), (Some(0), Some(0)));
        assert_eq!(both_styles("ua"), (Some(0), Some(0)));
        assert_eq!(both_styles("eo"), (Some(0), Some(0)));
    }

    #[test]
    fn qu_and_gi_mark_their_only_vowel() {
        assert_eq!(both_styles("qua"), (Some(0), Some(0)), "`quà`");
        assert_eq!(both_styles("quy"), (Some(0), Some(0)), "`quý`");
        assert_eq!(both_styles("quyên"), (Some(1), Some(1)), "`quyền`");
        assert_eq!(both_styles("gia"), (Some(0), Some(0)), "`già`");
        assert_eq!(both_styles("giư"), (Some(0), Some(0)), "`giữ`");
        assert_eq!(both_styles("giao"), (Some(0), Some(0)), "`giáo`");
    }

    #[test]
    fn a_syllable_without_a_nucleus_has_no_position() {
        // The parser never produces these, but tone placement must not be the place
        // where an impossible syllable panics.
        let empty = Syllable {
            onset_length: 1,
            nucleus: ['\0'; 3],
            nucleus_length: 0,
            coda_length: 0,
        };
        assert_eq!(tone_index(&empty, ToneStyle::New), None);
        // Closed and empty goes through a different branch (rule 2).
        let closed = Syllable {
            coda_length: 1,
            ..empty
        };
        assert_eq!(tone_index(&closed, ToneStyle::New), None);
        // More vowels than the array can hold is also refused rather than read.
        let overflowing = Syllable {
            nucleus_length: 4,
            ..empty
        };
        assert_eq!(tone_index(&overflowing, ToneStyle::New), None);
    }
}
