//! Splitting a word into the parts Vietnamese tone placement needs (spec §3.3).
//!
//! The parser works on the *transformed* word: quality diacritics (`ă â ê ô ơ ư`)
//! are already in it, tones are not. It answers one question — which vowel of the
//! nucleus takes the tone — and refuses everything that is not one syllable.
//!
//! There is deliberately no separate medial field. `qu` and `gi` are onsets, so
//! `quà` and `già` place the tone on their only vowel, and a glide stays in the
//! nucleus: `hoa` has two nucleus vowels (which is exactly why `hòa` and `hoà`
//! disagree) and `khuỷu` has three (which is why the tone lands on the middle one).
//! Splitting the glide out would give `hoa` one nucleus vowel and `khuỷu` two, and
//! both tone placements would then be wrong.

use crate::unicode::base_letter;

/// A Vietnamese syllable is at most seven letters (`nghiêng`, `khuyểnh`). The
/// parser reads into a fixed array, so no input length can make it allocate and an
/// over-long word is rejected instead of truncated.
pub(crate) const MAXIMUM_SYLLABLE_CHARACTERS: usize = 16;

/// A word split into onset, nucleus and coda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Syllable {
    /// Consonant letters before the first nucleus vowel, including the `u` of `qu`
    /// and the `i` of `gi`.
    pub(crate) onset_length: usize,
    /// The nucleus vowels as written, quality diacritics included. Slots past
    /// `nucleus_length` are unused.
    pub(crate) nucleus: [char; 3],
    pub(crate) nucleus_length: usize,
    /// Consonant letters after the last nucleus vowel.
    pub(crate) coda_length: usize,
}

impl Syllable {
    pub(crate) fn has_coda(&self) -> bool {
        self.coda_length > 0
    }
}

/// Split `word` into a syllable, or `None` when it is not exactly one syllable.
pub(crate) fn parse(word: &str) -> Option<Syllable> {
    let mut letters = ['\0'; MAXIMUM_SYLLABLE_CHARACTERS];
    let mut length = 0;
    for character in word.chars() {
        if length == MAXIMUM_SYLLABLE_CHARACTERS {
            return None;
        }
        letters[length] = character;
        length += 1;
    }
    if length == 0 {
        return None;
    }

    let mut onset_length = 0;
    while onset_length < length && is_consonant(letters[onset_length]) {
        onset_length += 1;
    }

    // `qu` and `gi` are onsets, not vowel clusters (spec §3.3). `gi` is only an
    // onset when a vowel follows it, so `gì` keeps `i` as its nucleus.
    let onset_starts_with_qu = onset_length == 1
        && letters[0].eq_ignore_ascii_case(&'q')
        && length > 1
        && letters[1].eq_ignore_ascii_case(&'u');
    let onset_starts_with_gi = onset_length == 1
        && letters[0].eq_ignore_ascii_case(&'g')
        && length > 2
        && letters[1].eq_ignore_ascii_case(&'i')
        && is_vowel(letters[2]);
    if onset_starts_with_qu || onset_starts_with_gi {
        onset_length = 2;
    }

    let mut nucleus_end = onset_length;
    while nucleus_end < length && is_vowel(letters[nucleus_end]) {
        nucleus_end += 1;
    }
    let nucleus_length = nucleus_end - onset_length;
    if !(1..=3).contains(&nucleus_length) {
        return None;
    }

    let mut coda_end = nucleus_end;
    while coda_end < length && is_consonant(letters[coda_end]) {
        coda_end += 1;
    }
    // A vowel after the coda means more than one syllable (`hello`), and a letter
    // that is neither vowel nor consonant (`1`, `á` before tones exist) is not part
    // of any syllable.
    if coda_end != length {
        return None;
    }
    let coda_length = length - nucleus_end;
    // Vietnamese codas are one or two letters; whether `xt` is a real coda is a
    // spelling question, not a parsing one (spell check arrives in T1.5).
    if coda_length > 2 {
        return None;
    }

    let mut nucleus = ['\0'; 3];
    for (slot, letter) in nucleus.iter_mut().zip(&letters[onset_length..nucleus_end]) {
        *slot = *letter;
    }

    Some(Syllable {
        onset_length,
        nucleus,
        nucleus_length,
        coda_length,
    })
}

/// Vowels are the letters that can carry a tone. `y` counts: `quý`, `khuya`.
fn is_vowel(character: char) -> bool {
    matches!(
        base_letter(character).to_ascii_lowercase(),
        'a' | 'e' | 'i' | 'o' | 'u' | 'y'
    )
}

fn is_consonant(character: char) -> bool {
    base_letter(character).is_ascii_alphabetic() && !is_vowel(character)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lengths(word: &str) -> Option<(usize, usize, usize)> {
        parse(word).map(|syllable| {
            (
                syllable.onset_length,
                syllable.nucleus_length,
                syllable.coda_length,
            )
        })
    }

    #[test]
    fn open_syllables() {
        assert_eq!(lengths("a"), Some((0, 1, 0)));
        assert_eq!(lengths("cai"), Some((1, 2, 0)));
        assert_eq!(lengths("cua"), Some((1, 2, 0)));
        assert_eq!(lengths("hoa"), Some((1, 2, 0)));
        // Tones do not change the structure: the tone table landed in T1.4, so a
        // toned vowel is a vowel like any other.
        assert_eq!(lengths("thuỷ"), Some((2, 2, 0)));
        assert_eq!(lengths("uy"), Some((0, 2, 0)));
    }

    #[test]
    fn closed_syllables() {
        assert_eq!(lengths("toan"), Some((1, 2, 1)));
        assert_eq!(lengths("tiêng"), Some((1, 2, 2)));
        assert_eq!(lengths("nghiêng"), Some((3, 2, 2)));
        assert_eq!(lengths("an"), Some((0, 1, 1)));
    }

    #[test]
    fn qu_and_gi_are_onsets() {
        assert_eq!(lengths("qua"), Some((2, 1, 0)));
        assert_eq!(lengths("quy"), Some((2, 1, 0)));
        assert_eq!(lengths("quyên"), Some((2, 2, 1)));
        assert_eq!(lengths("gia"), Some((2, 1, 0)));
        assert_eq!(lengths("giao"), Some((2, 2, 0)));
        // Without a following vowel, `i` is the nucleus, not part of an onset.
        assert_eq!(lengths("gi"), Some((1, 1, 0)));
        assert_eq!(lengths("gì"), Some((1, 1, 0)));
    }

    #[test]
    fn the_nucleus_keeps_its_quality_marks() {
        let syllable = parse("tiêng").expect("one syllable");
        assert_eq!(syllable.nucleus[..syllable.nucleus_length], ['i', 'ê']);
    }

    #[test]
    fn qu_and_gi_are_onsets_only_in_their_spelling() {
        // `q` on its own is not an onset followed by a vowel.
        assert_eq!(lengths("q"), None);
        // `qa` is not `qu`: it is the `u` that makes the onset.
        assert_eq!(lengths("qa"), Some((1, 1, 0)));
        // `gi` needs a vowel after the `i`, so `gin` keeps `i` in the nucleus.
        assert_eq!(lengths("gin"), Some((1, 1, 1)));
    }

    #[test]
    fn a_coda_longer_than_two_consonants_is_refused() {
        assert_eq!(lengths("anxt"), None);
    }

    #[test]
    fn rejects_words_that_are_not_one_syllable() {
        assert_eq!(parse(""), None);
        assert_eq!(lengths("ngh"), None, "no vowel");
        assert_eq!(lengths("hello"), None, "a vowel follows the coda");
        assert_eq!(lengths("aaaa"), None, "four nucleus vowels");
        assert_eq!(lengths("text"), Some((1, 1, 2)), "coda validity is T1.5");
        assert_eq!(lengths("12"), None);
        assert_eq!(lengths("ban1"), None);
        assert_eq!(lengths("aaaaaaaaaaaaaaaaa"), None, "longer than the cap");
    }

    #[test]
    fn byte_boundaries_do_not_matter() {
        // `ươ` is two characters in six bytes; the parser counts characters.
        assert_eq!(lengths("ươ"), Some((0, 2, 0)));
    }
}
