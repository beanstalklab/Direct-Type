//! Which letter sequences are Vietnamese syllables (spec §3.5).
//!
//! `auto_restore` and `spell_check` both need one answer — "could this word be
//! Vietnamese?" — and no other part of the engine does. The answer is a closed-set
//! membership test: onset, nucleus and coda each have a fixed inventory, and a word
//! is a syllable when all three are in it.
//!
//! The tables over-generate on purpose. A word wrongly called invalid is given back
//! to the user as raw keystrokes, which destroys diacritics they asked for; a word
//! wrongly called valid only keeps a transformation they may not want, which is what
//! the toggle key and `spell_check` are for. So there is no nucleus-to-coda
//! compatibility matrix — `tếc` passes even though Vietnamese does not spell it —
//! and every rare-but-real spelling is listed.
//!
//! Inventory source: `BambooEngine/bamboo-core` (`spelling.go`, MIT), whose
//! `firstConsonantSeqs`, `vowelSeqs` and `lastConsonantSeqs` hold the same closed
//! sets. The example words in the tests are the check that the lists are complete
//! enough for real Vietnamese, including `nghiêng`, `khuỷu` and `rượu`.

use crate::syllable::{self, MAXIMUM_SYLLABLE_CHARACTERS};
use crate::unicode::{letter_mark, tone_of, without_tone};

/// Onset consonants, longest spellings included. The empty onset is a syllable
/// like `ăn`, so it is a member rather than a special case.
const ONSETS: &[&str] = &[
    "", "b", "c", "ch", "d", "đ", "g", "gh", "gi", "h", "k", "kh", "l", "m", "n", "ng", "ngh",
    "nh", "p", "ph", "qu", "r", "s", "t", "th", "tr", "v", "x",
];

/// Nucleus vowels, written with the quality diacritic and without the tone.
const NUCLEI: &[&str] = &[
    // One vowel.
    "a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y",
    // Two vowels, with or without a coda.
    "ai", "ao", "au", "âu", "ay", "ây", "eo", "êu", "ia", "iê", "iu", "oa", "oă", "oe", "oi", "ôi",
    "ơi", "ua", "uâ", "uê", "ui", "uô", "uơ", "uy", "ưa", "ưi", "ưu", "ươ", "yê",
    // Three vowels, the last group Telex has to reach with one `w` (`ươu`).
    "iêu", "oai", "oao", "oay", "oeo", "uây", "uôi", "ươi", "ươu", "uya", "uyê", "uyu", "yêu",
];

/// Coda consonants. `k` is missing on purpose: it only ends loanwords (`ốk`), and
/// Vietnamese spelling uses `c` there (`ốc`).
const CODAS: &[&str] = &["", "c", "ch", "m", "n", "ng", "nh", "p", "t"];

/// Whether `word` is exactly one Vietnamese syllable.
pub(crate) fn is_valid_syllable(word: &str) -> bool {
    let mut letters = ['\0'; MAXIMUM_SYLLABLE_CHARACTERS];
    let Some(length) = syllable::copy_letters_into(word, &mut letters) else {
        return false;
    };
    let Some(parts) = split(&letters[..length]) else {
        return false;
    };
    is_spelled(parts.onset, ONSETS)
        && is_spelled(parts.nucleus, NUCLEI)
        && is_spelled(parts.coda, CODAS)
}

/// Whether `word` is a syllable or the beginning of one.
///
/// `spell_check` asks this while the user is still typing, so a prefix counts:
/// `ngh` is nothing yet, and `nghiêng` is one syllable. A word that can no longer
/// become a syllable — `tes` with its impossible coda, `hello` with a vowel after
/// the coda — is not a prefix of anything and is left alone.
pub(crate) fn can_become_valid_syllable(word: &str) -> bool {
    let mut letters = ['\0'; MAXIMUM_SYLLABLE_CHARACTERS];
    let Some(length) = syllable::copy_letters_into(word, &mut letters) else {
        return false;
    };
    let letters = &letters[..length];
    if !letters.iter().any(|letter| syllable::is_vowel(*letter)) {
        // Only an onset so far: `ng` on its way to `nghe`.
        return is_beginning_of_spelling(letters, ONSETS);
    }
    match split(letters) {
        Some(parts) => {
            is_spelled(parts.onset, ONSETS)
                && is_beginning_of_spelling(parts.nucleus, NUCLEI)
                && is_beginning_of_spelling(parts.coda, CODAS)
        }
        // `qu` is the one word with a vowel but no nucleus: its `u` already belongs
        // to the onset, and `qua` is still coming. Every other refusal — a vowel
        // after the coda, four nucleus vowels — cannot be repaired by typing more.
        None => is_beginning_of_spelling(letters, ONSETS),
    }
}

/// Whether `auto_restore` must give the user back the keystrokes `shown` came from.
///
/// Three guards keep the rewrite to the case it exists for, a word the engine turned
/// into something no Vietnamese speaker writes:
///
/// - `shown == raw_keys` means the transformation changed nothing, so there is
///   nothing to undo (`hello`, `telex`).
/// - No diacritic means the shown text is the typed text minus a modifier the user
///   undid: `aaa` shows `aa` and `ddd` shows `dd`, and both must survive.
/// - No vowel means there is no syllable to protect and no tone to lose: `dd` shows
///   `đ`, a letter rather than a mangled word.
///
/// What remains is a word that carries a mark, has a vowel, and is still not a
/// syllable, which is `asd` reaching `ád`.
pub(crate) fn restore_is_needed(shown: &str, raw_keys: &str) -> bool {
    shown != raw_keys && carries_diacritic(shown) && has_vowel(shown) && !is_valid_syllable(shown)
}

/// The parts of a word, borrowed from the caller's fixed buffer.
struct Parts<'a> {
    onset: &'a [char],
    nucleus: &'a [char],
    coda: &'a [char],
}

/// Split a word the way the syllable parser splits it.
fn split(letters: &[char]) -> Option<Parts<'_>> {
    let syllable = syllable::parse_letters(letters)?;
    let nucleus_end = syllable.onset_length + syllable.nucleus_length;
    Some(Parts {
        onset: &letters[..syllable.onset_length],
        nucleus: &letters[syllable.onset_length..nucleus_end],
        coda: &letters[nucleus_end..],
    })
}

/// Whether any letter of `text` carries a quality diacritic or a tone.
fn carries_diacritic(text: &str) -> bool {
    text.chars()
        .any(|letter| letter_mark(letter).is_some() || tone_of(letter).is_some())
}

fn has_vowel(text: &str) -> bool {
    text.chars().any(syllable::is_vowel)
}

/// The spelling the tables compare: lowercase, tone removed, quality mark kept.
fn normalized_letter(letter: char) -> char {
    without_tone(lowercase(letter))
}

/// `char::to_lowercase` yields at most three characters and never allocates, unlike
/// `str::to_lowercase`, so it is usable on the keystroke path.
fn lowercase(letter: char) -> char {
    letter.to_lowercase().next().unwrap_or(letter)
}

fn is_spelled(letters: &[char], spellings: &[&str]) -> bool {
    spellings
        .iter()
        .any(|spelling| is_spelled_as(letters, spelling))
}

/// Case- and tone-insensitive equality with one table entry.
fn is_spelled_as(letters: &[char], spelling: &str) -> bool {
    let mut expected = spelling.chars();
    for letter in letters {
        if expected.next() != Some(normalized_letter(*letter)) {
            return false;
        }
    }
    expected.next().is_none()
}

/// Whether `letters` is the start of a table entry, the whole entry included.
fn is_beginning_of_spelling(letters: &[char], spellings: &[&str]) -> bool {
    spellings.iter().any(|spelling| {
        let mut expected = spelling.chars();
        letters
            .iter()
            .all(|letter| expected.next() == Some(normalized_letter(*letter)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Words are listed in a string rather than an array so the table reads like the
    /// syllable inventory it checks, instead of one word per line.
    fn words(list: &str) -> impl Iterator<Item = &str> {
        list.split_whitespace()
    }

    #[test]
    fn real_words_are_syllables() {
        let list = "a ă â e ê i o ô ơ u ư y đi ăn ông ửng tét nét xét mét việt tiếng đường \
                    nguyễn thuyền khuỷu khuya oắt hoàng quyên già gì rượu ngoeo xoay khuây tuổi \
                    tươi người trời mưa mía mùa huế thuở yêu yên nghĩa nghiêng chuyện đây chưa \
                    Việt ĐƯỜNG";
        for word in words(list) {
            assert!(is_valid_syllable(word), "{word:?} should be a syllable");
        }
    }

    #[test]
    fn words_vietnamese_does_not_spell_are_not_syllables() {
        let list = "q đ ng ád tes text asd aaa awa aơ aư hello ban1 nghx";
        for word in words(list) {
            assert!(
                !is_valid_syllable(word),
                "{word:?} should not be a syllable"
            );
        }
        // The empty word is not a syllable either, and `split_whitespace` cannot say so.
        assert!(!is_valid_syllable(""));
    }

    #[test]
    fn every_part_of_the_word_is_checked() {
        // The onset is wrong, the nucleus and the coda are right.
        assert!(!is_valid_syllable("nxang"));
        // The onset and the coda are right, the nucleus is wrong: `ie` is not `iê`.
        assert!(!is_valid_syllable("tiec"));
        // The onset and the nucleus are right, the coda is wrong.
        assert!(!is_valid_syllable("canx"));
        // The tables are permissive about the combination, not about the parts: `c`
        // after `iê` is a real rhyme, so this one passes even though it is rare.
        assert!(is_valid_syllable("tiêc"));
    }

    #[test]
    fn a_nucleus_that_gained_a_tone_is_the_same_nucleus() {
        assert!(is_valid_syllable("toán"));
        assert!(is_valid_syllable("toan"));
        assert!(is_valid_syllable("hoà"));
        assert!(is_valid_syllable("hòa"));
    }

    #[test]
    fn prefixes_of_syllables_can_become_one() {
        let list = "q qu qua quy quyê ng ngh nghi nghiê nh đ đư đườ a ă â té tét vi viê việ th \
                    thu thươ uô ươ ươi";
        for word in words(list) {
            assert!(
                can_become_valid_syllable(word),
                "{word:?} should still be able to become a syllable"
            );
        }
        // An empty word is the beginning of every syllable.
        assert!(can_become_valid_syllable(""));
    }

    #[test]
    fn dead_ends_cannot_become_a_syllable() {
        let list = "aa aaa tes text hello awa aơ aư nghx dd ban1 ab nc";
        for word in words(list) {
            assert!(
                !can_become_valid_syllable(word),
                "{word:?} should be a dead end"
            );
        }
    }

    #[test]
    fn restore_takes_a_mangled_word_back_to_its_keystrokes() {
        assert!(restore_is_needed("ád", "asd"));
        assert!(restore_is_needed("àd", "asdf"));
        assert!(restore_is_needed("ôw", "oow"));
        // An undone modifier is usually not a syllable either, but the user typed
        // exactly what is shown, so there is nothing to undo.
        assert!(!restore_is_needed("aa", "aaa"));
        assert!(!restore_is_needed("aw", "aww"));
        assert!(!restore_is_needed("dd", "ddd"));
        // A letter without a vowel is not a mangled word.
        assert!(!restore_is_needed("đ", "dd"));
        // Nothing was transformed.
        assert!(!restore_is_needed("hello", "hello"));
        assert!(!restore_is_needed("sz", "sz"));
        // A syllable that English happens to spell: see the limit documented in
        // spec §3.5, `test` reaches `tét` and `tét` is real Vietnamese.
        assert!(!restore_is_needed("tét", "test"));
    }
}
