//! Telex transformation (spec §3.2 for letters and §3.4 for tones).
//!
//! The transformation is a fold over the raw keystrokes of one word, so a modifier
//! can rewrite a character the application has already committed: the caller
//! derives the delete-and-replace action from the difference. Nothing here holds a
//! composition buffer, which is what removes the underline.
//!
//! Letter modifiers rewrite the word as it is typed. A tone is different: which
//! vowel carries it depends on the whole syllable, so a tone key only records
//! *which* tone is wanted and the mark is placed once the word is complete. That is
//! also what lets `toasn` reach `toán`: the mark typed before the coda moves when
//! the coda arrives.

use crate::syllable::parse;
use crate::tone::tone_index;
use crate::unicode::{base_letter, compose, with_tone, LetterMark, Tone};
use crate::{Config, ToneStyle};

/// Where the trailing letter of the transformed word came from.
///
/// Doubling a modifier has to undo exactly what the previous keystroke did, and the
/// transformed text alone cannot tell `ww` (a lone `w` rendered as `ư`) from `uw`
/// (`u` plus a horn). Spec §3.2 requires `ww → w` and `aww → aw`, so the fold
/// carries this alongside the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// The previous keystroke added nothing to the trailing letter.
    Literal,
    /// The previous `w`, `[` or `]` key became the trailing letter itself.
    ModifierKey(char),
    /// The previous keystroke added a diacritic to the trailing letter.
    Marked {
        mark: LetterMark,
        /// The same keystroke also horned the `u` in front of an `o`.
        also_horned_u: bool,
    },
}

/// A tone a tone key asked for, waiting for the word to be complete.
#[derive(Debug, Clone, Copy)]
struct PendingTone {
    tone: Tone,
    /// Character position where the keystroke itself belongs if the tone is removed
    /// again: `asz` keeps the `s`, and in `asdz` that `s` returns before the `d`.
    letter_position: usize,
    letter: char,
}

/// Rewrite `output` to be the Telex transformation of `raw_keystrokes`.
///
/// `output` is cleared first and keeps its capacity, so a warmed-up engine does not
/// allocate per keystroke.
pub(crate) fn apply_telex(raw_keystrokes: &str, config: &Config, output: &mut String) {
    if fold(raw_keystrokes, config, true, output) {
        return;
    }
    // The finished word is not one syllable, so its tone keys were letters after
    // all: derive again with every one of them literal. That also keeps every
    // keystroke visible, which is the best attempt §3.5 asks for until T1.5 adds
    // `auto_restore`.
    fold(raw_keystrokes, config, false, output);
}

/// Fold `raw_keystrokes` into `output`.
///
/// `tone_keys_act` is false for the retry described in [`apply_telex`]: every tone
/// key is then an ordinary letter. The result is final when no tone is left
/// unplaced, which is what the bool reports.
fn fold(raw_keystrokes: &str, config: &Config, tone_keys_act: bool, output: &mut String) -> bool {
    output.clear();
    let last_letter_position = last_letter_position(raw_keystrokes);
    let mut origin = Origin::Literal;
    let mut pending_tone: Option<PendingTone> = None;
    let mut previous_tone_key: Option<char> = None;

    for (position, key) in raw_keystrokes.chars().enumerate() {
        let tone_key_acts = tone_keys_act
            && mark_may_be_typed_here(config.free_tone_mark, position, last_letter_position);
        if tone_key_acts && apply_tone_key(output, key, &mut pending_tone, &mut previous_tone_key) {
            // A tone key changes no letter, so a following letter modifier starts
            // from a literal trailing letter.
            origin = Origin::Literal;
            continue;
        }
        origin = append_letter(output, key, origin);
        previous_tone_key = None;
    }

    match pending_tone {
        None => true,
        Some(pending) => place_tone(output, pending.tone, config.tone_style),
    }
}

/// Handle one tone key. Returns whether it acted as a modifier.
///
/// A tone key is a letter until it is inside a syllable (`sz` keeps its `s`), and
/// doubling undoes it instead of applying a second tone (`ass → as`). Shift is not a
/// different key, so `ASs` undoes like `ass`.
fn apply_tone_key(
    output: &mut String,
    key: char,
    pending_tone: &mut Option<PendingTone>,
    previous_tone_key: &mut Option<char>,
) -> bool {
    if let Some(tone) = tone_of_key(key) {
        if parse(output.as_str()).is_none() {
            return false;
        }
        if previous_tone_key.is_some_and(|previous| previous.eq_ignore_ascii_case(&key)) {
            restore_tone_key_letter(output, pending_tone);
            *previous_tone_key = None;
            return true;
        }
        *pending_tone = Some(PendingTone {
            tone,
            letter_position: output.chars().count(),
            letter: key,
        });
        *previous_tone_key = Some(key);
        return true;
    }
    if is_tone_remover(key) && pending_tone.is_some() {
        restore_tone_key_letter(output, pending_tone);
        *previous_tone_key = None;
        return true;
    }
    false
}

/// Whether a tone key at `position` may act.
///
/// With `free_tone_mark` the mark may be typed anywhere inside the syllable. Without
/// it the mark has to follow the last letter of the word, which in Telex means after
/// the coda: `vieejt` stays `viêjt` while `vieetj` is `việt`.
fn mark_may_be_typed_here(
    free_tone_mark: bool,
    position: usize,
    last_letter_position: Option<usize>,
) -> bool {
    if free_tone_mark {
        return true;
    }
    match last_letter_position {
        Some(last) => position > last,
        None => true,
    }
}

/// The tone a Telex key asks for, in either case.
fn tone_of_key(key: char) -> Option<Tone> {
    match key.to_ascii_lowercase() {
        's' => Some(Tone::Acute),
        'f' => Some(Tone::Grave),
        'r' => Some(Tone::HookAbove),
        'x' => Some(Tone::Tilde),
        'j' => Some(Tone::UnderDot),
        _ => None,
    }
}

/// `z` removes whatever tone the word carries.
fn is_tone_remover(key: char) -> bool {
    key.eq_ignore_ascii_case(&'z')
}

/// Position of the last raw key that is a letter rather than a tone key.
///
/// Only `free_tone_mark` off needs it, and it is the reason that setting can be
/// decided from the raw keystrokes alone instead of from the typing history.
fn last_letter_position(raw_keystrokes: &str) -> Option<usize> {
    raw_keystrokes
        .chars()
        .enumerate()
        .filter(|(_, key)| tone_of_key(*key).is_none() && !is_tone_remover(*key))
        .map(|(position, _)| position)
        .last()
}

/// Put the letter of the tone key that asked for the pending tone back where the key
/// was typed, and forget the tone.
fn restore_tone_key_letter(output: &mut String, pending_tone: &mut Option<PendingTone>) {
    let Some(pending) = pending_tone.take() else {
        return;
    };
    match output.char_indices().nth(pending.letter_position) {
        Some((byte_index, _)) => output.insert(byte_index, pending.letter),
        None => output.push(pending.letter),
    }
}

/// Put `tone` on the vowel the syllable rules choose.
///
/// Returns false when the word is not one syllable, which is how the caller learns
/// that its tone keys were letters.
fn place_tone(output: &mut String, tone: Tone, tone_style: ToneStyle) -> bool {
    let Some(syllable) = parse(output.as_str()) else {
        return false;
    };
    let Some(index) = tone_index(&syllable, tone_style) else {
        return false;
    };
    let Some((byte_index, character)) = output.char_indices().nth(syllable.onset_length + index)
    else {
        return false;
    };
    let Some(toned) = with_tone(character, tone) else {
        return false;
    };
    let mut encoded = [0u8; 4];
    output.replace_range(
        byte_index..byte_index + character.len_utf8(),
        toned.encode_utf8(&mut encoded),
    );
    true
}

/// Apply one letter keystroke and report how the trailing letter now came about.
fn append_letter(output: &mut String, key: char, origin: Origin) -> Origin {
    match key {
        // `[` and `]` are letters in their own right: §3.2 lists them as `ơ` and
        // `ư`, never as a change to a preceding letter, unlike `aw → ă`.
        '[' | ']' => append_bracket(output, key, origin),
        _ => match key.to_ascii_lowercase() {
            'w' => append_w(output, key, origin),
            'a' | 'e' | 'o' => append_circumflex(output, key, origin),
            'd' => append_stroke(output, key, origin),
            _ => append_literal(output, key),
        },
    }
}

/// Append a character that no rule claims.
fn append_literal(output: &mut String, key: char) -> Origin {
    output.push(key);
    Origin::Literal
}

/// `aa → â`, `ee → ê`, `oo → ô`.
fn append_circumflex(output: &mut String, key: char, origin: Origin) -> Origin {
    let Some(previous) = output.chars().next_back() else {
        return append_literal(output, key);
    };
    // `e` after `o` is two letters: only the same base letter takes the mark.
    if !base_letter(previous).eq_ignore_ascii_case(&key) {
        return append_literal(output, key);
    }
    match mark_trailing(output, LetterMark::Circumflex, key, origin) {
        Some(new_origin) => new_origin,
        // `ă` + `a` is not `â`; the key stays literal and the word keeps its mark.
        None => append_literal(output, key),
    }
}

/// `dd → đ`.
fn append_stroke(output: &mut String, key: char, origin: Origin) -> Origin {
    let Some(previous) = output.chars().next_back() else {
        return append_literal(output, key);
    };
    if !base_letter(previous).eq_ignore_ascii_case(&'d') {
        return append_literal(output, key);
    }
    match mark_trailing(output, LetterMark::Stroke, key, origin) {
        Some(new_origin) => new_origin,
        None => append_literal(output, key),
    }
}

/// `w → ư`, `aw → ă`, `uw → ư`, `ow → ơ`, and `uow → ươ`.
fn append_w(output: &mut String, key: char, origin: Origin) -> Origin {
    // `ww → w`: a second `w` cancels the letter the first one produced. The letter
    // that stays is the one already on screen, case included, because an undo must not
    // also change what the user sees (`Ww → W`, not `w`).
    if let Origin::ModifierKey(produced) = origin {
        if produced.eq_ignore_ascii_case(&'w') {
            output.pop();
            output.push(produced);
            return Origin::Literal;
        }
    }

    let Some(previous) = output.chars().next_back() else {
        output.push(horned_u_with_case(key));
        return Origin::ModifierKey(key);
    };

    let applied = match base_letter(previous).to_ascii_lowercase() {
        'a' => mark_trailing(output, LetterMark::Breve, key, origin),
        'u' => mark_trailing(output, LetterMark::Horn, key, origin),
        'o' => horn_u_o_pair(output, key, origin),
        // After a consonant, `w` is a letter of its own.
        _ => {
            output.push(horned_u_with_case(key));
            return Origin::ModifierKey(key);
        }
    };
    match applied {
        Some(new_origin) => new_origin,
        // `ô` + `w` cannot gain a horn; keeping the key literal beats inventing a
        // letter that Vietnamese does not spell.
        None => append_literal(output, key),
    }
}

/// `[ → ơ`, `] → ư`, and `[[ → [` on the same undo rule as `ww`.
fn append_bracket(output: &mut String, key: char, origin: Origin) -> Origin {
    if origin == Origin::ModifierKey(key) {
        output.pop();
        output.push(key);
        return Origin::Literal;
    }
    output.push(if key == '[' { 'ơ' } else { 'ư' });
    Origin::ModifierKey(key)
}

/// Add `mark` to the trailing letter, unless the previous keystroke already added
/// exactly that mark.
///
/// Doubling a modifier undoes it and keeps the key literal (`aaa → aa`,
/// `ddd → dd`). `None` means this key is not a modifier here and the caller must
/// keep it literal.
fn mark_trailing(
    output: &mut String,
    mark: LetterMark,
    key: char,
    origin: Origin,
) -> Option<Origin> {
    if matches!(origin, Origin::Marked { mark: applied, .. } if applied == mark) {
        undo_mark(output, origin, key);
        return Some(Origin::Literal);
    }
    let previous = output.chars().next_back()?;
    let marked = compose(previous, mark)?;
    output.pop();
    output.push(marked);
    Some(Origin::Marked {
        mark,
        also_horned_u: false,
    })
}

/// `w` after `o`: horn the `o` and the `u` in front of it.
///
/// `ươ` is otherwise unreachable in one keystroke, and every Telex user types that
/// diphthong as `uow`. Undoing restores both letters, so `uoww` is `uow`.
fn horn_u_o_pair(output: &mut String, key: char, origin: Origin) -> Option<Origin> {
    if matches!(
        origin,
        Origin::Marked {
            mark: LetterMark::Horn,
            ..
        }
    ) {
        undo_mark(output, origin, key);
        return Some(Origin::Literal);
    }

    let horned_o = compose(output.chars().next_back()?, LetterMark::Horn)?;
    output.pop();

    let mut also_horned_u = false;
    if let Some(before_o) = output.chars().next_back() {
        if base_letter(before_o).eq_ignore_ascii_case(&'u') {
            if let Some(horned_u) = compose(before_o, LetterMark::Horn) {
                output.pop();
                output.push(horned_u);
                also_horned_u = true;
            }
        }
    }
    output.push(horned_o);

    Some(Origin::Marked {
        mark: LetterMark::Horn,
        also_horned_u,
    })
}

/// Undo the diacritic the previous keystroke added and keep `key` literal.
fn undo_mark(output: &mut String, origin: Origin, key: char) {
    let horned_both = matches!(
        origin,
        Origin::Marked {
            also_horned_u: true,
            ..
        }
    );
    if horned_both {
        // One keystroke marked two letters, so undoing it restores both.
        if let Some(horned_o) = output.pop() {
            if let Some(horned_u) = output.pop() {
                output.push(base_letter(horned_u));
            }
            output.push(base_letter(horned_o));
        }
    } else if let Some(marked) = output.pop() {
        output.push(base_letter(marked));
    }
    output.push(key);
}

/// `ư` in the case of the key that produced it.
fn horned_u_with_case(key: char) -> char {
    if key.is_ascii_uppercase() {
        'Ư'
    } else {
        'ư'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform(raw_keystrokes: &str) -> String {
        let mut output = String::new();
        apply_telex(raw_keystrokes, &Config::default(), &mut output);
        output
    }

    #[test]
    fn single_letter_modifiers() {
        let cases = [
            ("aa", "â"),
            ("ee", "ê"),
            ("oo", "ô"),
            ("aw", "ă"),
            ("ow", "ơ"),
            ("uw", "ư"),
            ("dd", "đ"),
            ("w", "ư"),
            ("[", "ơ"),
            ("]", "ư"),
        ];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn doubling_undoes_the_modifier() {
        let cases = [
            ("aaa", "aa"),
            ("eee", "ee"),
            ("ooo", "oo"),
            ("aww", "aw"),
            ("oww", "ow"),
            ("uww", "uw"),
            ("ddd", "dd"),
            ("ww", "w"),
            ("[[", "["),
            ("]]", "]"),
        ];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn doubling_keeps_the_case_of_the_letter_on_screen() {
        // The undo removes the mark, not the case: `Ư` becomes `W`, so the letter on
        // screen never flips case while the user is removing a diacritic. The key that
        // comes back is the one the first press typed, not the one that undid it.
        let cases = [
            ("Ww", "W"),
            ("wW", "w"),
            ("WW", "W"),
            // A `w` that marked a letter leaves the letter and the current key, so the
            // key that comes back there is the one that undid the mark.
            ("AwW", "AW"),
            ("AWw", "Aw"),
            ("aWW", "aW"),
        ];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn horn_covers_both_letters_of_the_uo_pair() {
        assert_eq!(transform("uow"), "ươ");
        assert_eq!(transform("uwow"), "ươ");
        assert_eq!(transform("uoww"), "uow");
        assert_eq!(transform("dduowng"), "đương");
    }

    #[test]
    fn case_follows_the_letter_that_carries_the_mark() {
        assert_eq!(transform("AA"), "Â");
        assert_eq!(transform("Aa"), "Â");
        assert_eq!(transform("aA"), "â");
        assert_eq!(transform("W"), "Ư");
        assert_eq!(transform("DD"), "Đ");
        assert_eq!(transform("VIEE"), "VIÊ");
    }

    #[test]
    fn unclaimed_letters_pass_through() {
        assert_eq!(transform("hello"), "hello");
        assert_eq!(transform("ubuntu"), "ubuntu");
        assert_eq!(transform("0123456789"), "0123456789");
    }

    #[test]
    fn a_different_mark_keeps_the_key_literal() {
        // `ă` cannot take a circumflex, so the third vowel is just a letter.
        assert_eq!(transform("awa"), "ăa");
        // `ô` cannot take a horn.
        assert_eq!(transform("oow"), "ôw");
    }

    #[test]
    fn tone_keys_place_the_mark_once_the_word_is_complete() {
        // The tone key is typed before the coda, so at that moment only `toa`
        // exists; the mark has to move to the last nucleus vowel when `n` arrives.
        assert_eq!(transform("toasn"), "toán");
        assert_eq!(transform("vieejt"), "việt");
    }

    #[test]
    fn a_tone_key_outside_a_syllable_is_a_letter() {
        assert_eq!(transform("sz"), "sz");
        assert_eq!(transform("telex"), "telex");
    }

    #[test]
    fn a_tone_that_cannot_be_placed_keeps_every_keystroke() {
        // `ashello` is not one syllable, so `s` was a letter and must not vanish.
        assert_eq!(transform("ashello"), "ashello");
    }

    #[test]
    fn doubling_a_tone_key_ignores_the_letter_case() {
        // Shift is not a different key: the user pressed the same key twice, so the
        // mark has to come off. The letter put back at the position is the one the
        // pending tone was typed with, so `ASs` gives `AS` and `AsS` gives `As`.
        let cases = [("ASs", "AS"), ("AsS", "As"), ("asFf", "aF"), ("aSfF", "af")];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn without_free_tone_mark_only_a_trailing_mark_acts() {
        let mut output = String::new();
        let config = Config {
            free_tone_mark: false,
            ..Config::default()
        };
        apply_telex("vieejt", &config, &mut output);
        assert_eq!(output, "viêjt");
        apply_telex("vieetj", &config, &mut output);
        assert_eq!(output, "việt");
    }
}
