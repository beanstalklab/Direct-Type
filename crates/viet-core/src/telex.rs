//! The Telex rule pack (spec §3.2).
//!
//! Only two questions live here: which keys are tone keys, and what a letter keystroke
//! does to the trailing letter. The fold that walks the raw keys is
//! [`crate::input`], shared with VNI.
//!
//! Letter modifiers rewrite the word as it is typed. A tone is different: which vowel
//! carries it depends on the whole syllable, so a tone key only records *which* tone is
//! wanted and the fold places the mark once the word is complete.

use crate::input::{append_literal, horn_u_o_pair, mark_trailing, Origin, RulePack};
use crate::unicode::{base_letter, LetterMark, Tone};

/// Full Telex: tone keys `s f r x j`, `z` removes a tone, and `w`, `aa`, `ee`, `oo`,
/// `dd`, `[`, `]` add the quality diacritics.
pub(crate) struct Telex;

impl RulePack for Telex {
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
    fn removes_tone(key: char) -> bool {
        key.eq_ignore_ascii_case(&'z')
    }

    /// Telex tone keys are letters, so removing a tone gives the letter back: `asz` is
    /// `as`, not `a`.
    const RESTORES_TONE_KEY_LETTER: bool = true;

    fn append(output: &mut String, key: char, origin: Origin, more_letters_follow: bool) -> Origin {
        append_letter(output, key, origin, more_letters_follow)
    }
}

/// Apply one letter keystroke and report how the trailing letter now came about.
fn append_letter(
    output: &mut String,
    key: char,
    origin: Origin,
    more_letters_follow: bool,
) -> Origin {
    match key {
        // `[` and `]` are letters in their own right: §3.2 lists them as `ơ` and
        // `ư`, never as a change to a preceding letter, unlike `aw → ă`.
        '[' | ']' => append_bracket(output, key, origin),
        _ => match key.to_ascii_lowercase() {
            'w' => append_w(output, key, origin, more_letters_follow),
            'a' | 'e' | 'o' => append_circumflex(output, key, origin),
            'd' => append_stroke(output, key, origin),
            _ => append_literal(output, key),
        },
    }
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

/// `w → ư`, `aw → ă`, `uw → ư`, `ow → ơ`, `uow → uơ`, and `uowng → ương`.
fn append_w(output: &mut String, key: char, origin: Origin, more_letters_follow: bool) -> Origin {
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
        'o' => horn_u_o_pair(output, key, origin, more_letters_follow),
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
    use crate::{input, Config};

    fn transform(raw_keystrokes: &str) -> String {
        let mut output = String::new();
        input::apply::<Telex>(raw_keystrokes, &Config::default(), &mut output);
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
    fn horn_covers_both_letters_of_the_uo_pair_when_the_word_goes_on() {
        // `ương` has to be reachable in one keystroke…
        assert_eq!(transform("uowng"), "ương");
        assert_eq!(transform("dduowng"), "đương");
        assert_eq!(transform("uwow"), "ươ");
        // …while a word that ends at the `w` keeps `uơ`, which is the rhyme of
        // `thuở`, `quơ` and `huơ`.
        assert_eq!(transform("uow"), "uơ");
        assert_eq!(transform("thuow"), "thuơ");
        assert_eq!(transform("thuowr"), "thuở");
        assert_eq!(transform("huow"), "huơ");
        assert_eq!(transform("quow"), "quơ");
        // Undoing puts back whatever the keystroke marked. The second `w` is itself a
        // letter, so the first one horns both and the undo restores both.
        assert_eq!(transform("uoww"), "uow");
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
        input::apply::<Telex>("vieejt", &config, &mut output);
        assert_eq!(output, "viêjt");
        input::apply::<Telex>("vieetj", &config, &mut output);
        assert_eq!(output, "việt");
    }
}
