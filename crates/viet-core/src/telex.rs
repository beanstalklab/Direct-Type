//! Telex letter modifiers (spec §3.2).
//!
//! The transformation is a fold over the raw keystrokes of one word, so a modifier
//! can rewrite a character the application has already committed: the caller
//! derives the delete-and-replace action from the difference. Nothing here holds a
//! composition buffer, which is what removes the underline.

use crate::unicode::{base_letter, compose, LetterMark};

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

/// Rewrite `output` to be the Telex letter transformation of `raw_keystrokes`.
///
/// `output` is cleared first and keeps its capacity, so a warmed-up engine does not
/// allocate per keystroke.
pub(crate) fn apply_letter_modifiers(raw_keystrokes: &str, output: &mut String) {
    output.clear();
    let mut origin = Origin::Literal;
    for key in raw_keystrokes.chars() {
        origin = append_key(output, key, origin);
    }
}

/// Apply one keystroke and report how the trailing letter now came about.
fn append_key(output: &mut String, key: char, origin: Origin) -> Origin {
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
    // `ww → w`: a second `w` cancels the letter the first one produced.
    if origin == Origin::ModifierKey('w') || origin == Origin::ModifierKey('W') {
        output.pop();
        output.push(key);
        return Origin::Literal;
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
        apply_letter_modifiers(raw_keystrokes, &mut output);
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
}
