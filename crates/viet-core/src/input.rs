//! Folding one word's raw keystrokes into text (spec §3.2 for the rule packs, §3.4
//! for tone placement).
//!
//! Every method runs through this fold: it walks the raw keys, holds the tone a tone
//! key asked for until the word is finished, and lets a letter modifier rewrite the
//! trailing letter in place. Only three questions differ between methods — which key
//! asks for which tone, which key removes a tone, and what a key does to the trailing
//! letter — so a method is a [`RulePack`] and nothing else. `tone.rs` stays free of
//! method-specific branches because of that split.
//!
//! Nothing here holds a composition buffer: the transformed text is re-derived from
//! the raw keys on every keystroke, and the caller turns the difference into a
//! delete-and-replace action. That is what removes the underline.

use crate::syllable::parse;
use crate::tone::tone_index;
use crate::unicode::{base_letter, compose, with_tone, LetterMark, Tone};
use crate::{Config, ToneStyle};

/// What one input method does with a key.
pub(crate) trait RulePack {
    /// The tone `key` asks for, or `None` when it is not a tone key.
    fn tone_of_key(key: char) -> Option<Tone>;

    /// Whether `key` removes the tone the word carries.
    fn removes_tone(key: char) -> bool;

    /// Whether the character of a tone key goes back into the word when the tone is
    /// removed. Telex tone keys are letters, so `asz` keeps its `s`; VNI tone keys are
    /// digits, and a digit left behind would be noise, so `a10` is `a`.
    const RESTORES_TONE_KEY_LETTER: bool;

    /// Apply `key` to the trailing letter of `output` and report how the trailing
    /// letter now came about.
    ///
    /// `more_letters_follow` says whether another letter key is still to come in this
    /// word. Only the `uo` pair needs it: `uowng` is `ương`, while a word that ends at
    /// the `w` keeps `uơ` (`thuow → thuơ`), because `thuở`, `quơ` and `huơ` are the
    /// only words with that rhyme.
    fn append(output: &mut String, key: char, origin: Origin, more_letters_follow: bool) -> Origin;
}

/// Where the trailing letter of the transformed word came from.
///
/// Doubling a modifier has to undo exactly what the previous keystroke did, and the
/// transformed text alone cannot tell `ww` (a lone `w` rendered as `ư`) from `uw` (`u`
/// plus a horn). Spec §3.2 requires `ww → w` and `aww → aw`, so the fold carries this
/// alongside the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// The previous keystroke added nothing to the trailing letter.
    Literal,
    /// The previous key became the trailing letter itself (`w` → `ư`, `[` → `ơ`).
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
pub(crate) struct PendingTone {
    tone: Tone,
    /// Character position where the keystroke itself belongs if the tone is removed
    /// again: `asz` keeps the `s`, and in `asdz` that `s` returns before the `d`.
    letter_position: usize,
    letter: char,
}

/// Rewrite `output` to be the transformation of `raw_keystrokes` under `R`.
///
/// `output` is cleared first and keeps its capacity, so a warmed-up engine does not
/// allocate per keystroke.
pub(crate) fn apply<R: RulePack>(raw_keystrokes: &str, config: &Config, output: &mut String) {
    if fold::<R>(raw_keystrokes, config, true, output) {
        return;
    }
    // The finished word is not one syllable, so its tone keys were letters after all:
    // derive again with every one of them literal. That keeps every keystroke visible,
    // which is the best attempt §3.5 asks for while `auto_restore` is off.
    fold::<R>(raw_keystrokes, config, false, output);
}

/// Fold `raw_keystrokes` into `output`.
///
/// `tone_keys_act` is false for the retry described in [`apply`]: every tone key is
/// then an ordinary character. The result is final when no tone is left unplaced,
/// which is what the bool reports.
fn fold<R: RulePack>(
    raw_keystrokes: &str,
    config: &Config,
    tone_keys_act: bool,
    output: &mut String,
) -> bool {
    output.clear();
    let last_letter_position = last_letter_position::<R>(raw_keystrokes);
    let mut origin = Origin::Literal;
    let mut pending_tone: Option<PendingTone> = None;
    let mut previous_tone_key: Option<char> = None;

    for (position, key) in raw_keystrokes.chars().enumerate() {
        let tone_key_acts = tone_keys_act
            && mark_may_be_typed_here(config.free_tone_mark, position, last_letter_position);
        if tone_key_acts
            && apply_tone_key::<R>(output, key, &mut pending_tone, &mut previous_tone_key)
        {
            // A tone key changes no letter, so a following letter modifier starts from
            // a literal trailing letter.
            origin = Origin::Literal;
            continue;
        }
        let more_letters_follow = last_letter_position.is_some_and(|last| position < last);
        origin = R::append(output, key, origin, more_letters_follow);
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
fn apply_tone_key<R: RulePack>(
    output: &mut String,
    key: char,
    pending_tone: &mut Option<PendingTone>,
    previous_tone_key: &mut Option<char>,
) -> bool {
    if let Some(tone) = R::tone_of_key(key) {
        if parse(output.as_str()).is_none() {
            return false;
        }
        if previous_tone_key.is_some_and(|previous| previous.eq_ignore_ascii_case(&key)) {
            // The same key twice types the character again: one copy stays in the word.
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
    if R::removes_tone(key) && pending_tone.is_some() {
        if R::RESTORES_TONE_KEY_LETTER {
            restore_tone_key_letter(output, pending_tone);
        } else {
            *pending_tone = None;
        }
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

/// Position of the last raw key that is a letter rather than a tone key.
///
/// Only `free_tone_mark` off needs it, and it is the reason that setting can be
/// decided from the raw keystrokes alone instead of from the typing history.
fn last_letter_position<R: RulePack>(raw_keystrokes: &str) -> Option<usize> {
    raw_keystrokes
        .chars()
        .enumerate()
        .filter(|(_, key)| R::tone_of_key(*key).is_none() && !R::removes_tone(*key))
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

/// Append a character that no rule of the method claims.
pub(crate) fn append_literal(output: &mut String, key: char) -> Origin {
    output.push(key);
    Origin::Literal
}

/// Add `mark` to the trailing letter, unless the previous keystroke already added
/// exactly that mark.
///
/// Doubling a modifier undoes it and keeps the key literal (`aaa → aa`, `ddd → dd`,
/// `a66 → a6`). `None` means this key is not a modifier here and the caller must keep
/// it literal.
pub(crate) fn mark_trailing(
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

/// `w` or `7` after `o`: horn the `o`, and the `u` in front of it when the syllable
/// goes on.
///
/// `ương` needs one keystroke, so a `u` in front of the horned `o` is horned too while
/// another letter is still coming (`uowng → ương`). A word that ends at the horn key
/// keeps `uơ` (`thuow → thuơ`, `huow → huơ`, `quow → quơ`), because that is the only
/// rhyme `thuở` and `quơ` have. Undoing restores whatever the keystroke marked, so
/// `uoww` is `uow` and `uo77` is `uo7`.
pub(crate) fn horn_u_o_pair(
    output: &mut String,
    key: char,
    origin: Origin,
    also_horn_u: bool,
) -> Option<Origin> {
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
    if also_horn_u {
        if let Some(before_o) = output.chars().next_back() {
            if base_letter(before_o).eq_ignore_ascii_case(&'u') {
                if let Some(horned_u) = compose(before_o, LetterMark::Horn) {
                    output.pop();
                    output.push(horned_u);
                    also_horned_u = true;
                }
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
pub(crate) fn undo_mark(output: &mut String, origin: Origin, key: char) {
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
