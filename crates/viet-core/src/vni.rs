//! The VNI rule pack (spec §3.2).
//!
//! Digits carry everything: `1`–`5` are the tones, `0` removes the tone, and `6`–`9`
//! add the quality diacritics. The table is the one `fcitx5-bamboo` ships
//! (`input_method_def.go`, MIT): `6` is `AEO_ÂÊÔ`, `7` is `UO_ƯƠ`, `8` is `A_Ă`, `9` is
//! `D_Đ`. A digit that cannot do its job stays a digit (`n6`), because a keystroke that
//! silently disappears is worse than one that is visible.
//!
//! `7` on a `uo` pair horns both letters, as Telex's `w` does: `uo7 → ươ`. It is the
//! only one-keystroke way to reach that diphthong, and VNI users type it that way.

use crate::input::{append_literal, horn_u_o_pair, mark_trailing, Origin, RulePack};
use crate::unicode::{base_letter, LetterMark, Tone};

/// VNI: digits `1`–`5` for tones, `0` removes, `6`–`9` for the quality diacritics.
pub(crate) struct Vni;

impl RulePack for Vni {
    fn tone_of_key(key: char) -> Option<Tone> {
        match key {
            '1' => Some(Tone::Acute),
            '2' => Some(Tone::Grave),
            '3' => Some(Tone::HookAbove),
            '4' => Some(Tone::Tilde),
            '5' => Some(Tone::UnderDot),
            _ => None,
        }
    }

    /// `0` removes whatever tone the word carries.
    fn removes_tone(key: char) -> bool {
        key == '0'
    }

    /// A VNI tone key is a digit, and a digit left behind in a word is noise, so
    /// removing a tone drops it: `a10` is `a`, not `a1`. Doubling is different — the
    /// user typed the digit again, so one copy stays (`a11` is `a1`).
    const RESTORES_TONE_KEY_LETTER: bool = false;

    fn append(output: &mut String, key: char, origin: Origin, more_letters_follow: bool) -> Origin {
        match key {
            '6' => append_mark(output, key, origin, LetterMark::Circumflex, "aeo"),
            '7' => append_horn(output, key, origin, more_letters_follow),
            '8' => append_mark(output, key, origin, LetterMark::Breve, "a"),
            '9' => append_mark(output, key, origin, LetterMark::Stroke, "d"),
            _ => append_literal(output, key),
        }
    }
}

/// `a6 → â`, `e6 → ê`, `o6 → ô`, `a8 → ă`, `d9 → đ`.
fn append_mark(
    output: &mut String,
    key: char,
    origin: Origin,
    mark: LetterMark,
    letters: &str,
) -> Origin {
    let takes_mark = output
        .chars()
        .next_back()
        .is_some_and(|previous| letters.contains(base_letter(previous).to_ascii_lowercase()));
    if !takes_mark {
        return append_literal(output, key);
    }
    match mark_trailing(output, mark, key, origin) {
        Some(new_origin) => new_origin,
        // The letter already carries a different mark (`â` + `8`): keeping the digit
        // literal beats inventing a letter Vietnamese does not spell.
        None => append_literal(output, key),
    }
}

/// `u7 → ư`, `o7 → ơ`, and `uo7 → ươ`.
///
/// Unlike Telex's `w`, a digit never inserts a letter: there is nothing to insert
/// without a vowel in front of it, so a lone `7` stays a digit.
fn append_horn(
    output: &mut String,
    key: char,
    origin: Origin,
    more_letters_follow: bool,
) -> Origin {
    let Some(previous) = output.chars().next_back() else {
        return append_literal(output, key);
    };
    let applied = match base_letter(previous).to_ascii_lowercase() {
        'u' => mark_trailing(output, LetterMark::Horn, key, origin),
        'o' => horn_u_o_pair(output, key, origin, more_letters_follow),
        _ => return append_literal(output, key),
    };
    match applied {
        Some(new_origin) => new_origin,
        None => append_literal(output, key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{input, Config};

    fn transform(raw_keystrokes: &str) -> String {
        let mut output = String::new();
        input::apply::<Vni>(raw_keystrokes, &Config::default(), &mut output);
        output
    }

    #[test]
    fn digits_add_the_quality_diacritics() {
        let cases = [
            ("a6", "â"),
            ("e6", "ê"),
            ("o6", "ô"),
            ("a8", "ă"),
            ("o7", "ơ"),
            ("u7", "ư"),
            ("d9", "đ"),
            ("A6", "Â"),
            ("U7", "Ư"),
            ("D9", "Đ"),
        ];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn a_digit_that_cannot_do_its_job_stays_a_digit() {
        // `n` cannot take the circumflex, `6a` has nothing to mark, and `7` never
        // inserts a letter the way Telex's `w` does. A visible digit beats a keystroke
        // that disappears.
        let cases = [
            ("n6", "n6"),
            ("6a", "6a"),
            ("7", "7"),
            ("b8", "b8"),
            ("k9", "k9"),
            // The letter already carries a different mark.
            ("a67", "â7"),
            ("a68", "â8"),
        ];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn doubling_a_digit_undoes_the_mark_and_keeps_one_digit() {
        let cases = [
            ("a66", "a6"),
            ("e66", "e6"),
            ("o77", "o7"),
            ("u77", "u7"),
            ("a88", "a8"),
            ("d99", "d9"),
            ("uo77", "uo7"),
        ];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn seven_horns_both_letters_of_the_uo_pair_when_the_word_goes_on() {
        // The same rule as Telex's `w`: the pair is horned while another letter is
        // coming, and a word that ends at the `7` keeps the `uơ` rhyme of `thuở`.
        assert_eq!(transform("uo7ng"), "ương");
        assert_eq!(transform("d9uo7ng"), "đương");
        assert_eq!(transform("u7o7"), "ươ");
        assert_eq!(transform("uo7"), "uơ");
        assert_eq!(transform("thuo7"), "thuơ");
        assert_eq!(transform("uo77"), "uo7");
    }

    #[test]
    fn tone_digits_place_the_mark_on_the_finished_syllable() {
        let cases = [
            ("a1", "á"),
            ("a2", "à"),
            ("a3", "ả"),
            ("a4", "ã"),
            ("a5", "ạ"),
            ("ban1", "bán"),
            ("vie6t5", "việt"),
            ("vie65t", "việt"),
            ("toan1", "toán"),
            ("a16", "ấ"),
            ("hoa2", "hòa"),
        ];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn a_tone_digit_outside_a_syllable_is_a_digit() {
        assert_eq!(transform("1"), "1");
        assert_eq!(transform("1a"), "1a");
        assert_eq!(transform("ban1x"), "bánx");
    }

    #[test]
    fn zero_removes_the_tone_without_leaving_the_digit() {
        // The tone digit was consumed by the tone, and a digit coming back would be
        // noise: `a10` is `a`. Doubling differs, because there the user typed the
        // digit again (`a11` is `a1`).
        let cases = [
            ("a10", "a"),
            ("a50", "a"),
            ("vie6t50", "viêt"),
            ("a11", "a1"),
        ];
        for (raw, expected) in cases {
            assert_eq!(transform(raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn without_free_tone_mark_only_a_trailing_tone_acts() {
        let config = Config {
            free_tone_mark: false,
            ..Config::default()
        };
        let mut output = String::new();
        input::apply::<Vni>("vie65t", &config, &mut output);
        assert_eq!(output, "viê5t");
        input::apply::<Vni>("vie6t5", &config, &mut output);
        assert_eq!(output, "việt");
    }

    #[test]
    fn telex_keys_are_ordinary_letters_and_digits_here() {
        // The rule packs must not leak: `w`, `[`, `]`, `s` and `z` do nothing in VNI.
        for raw in ["uow", "aw", "dd", "as", "az", "[", "]"] {
            assert_eq!(transform(raw), raw, "raw {raw:?}");
        }
    }
}
