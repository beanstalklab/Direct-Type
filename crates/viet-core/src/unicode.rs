//! The closed set of Vietnamese letters the engine can produce.
//!
//! The engine never calls a Unicode library: the characters a Vietnamese syllable
//! can contain are a small closed set, so the mapping is a static table (decision
//! D1 in `docs/SPEC-engine-core.md`). Tones arrive in T1.4 and extend the same
//! table; today it holds only the letter marks that Telex letter modifiers use.

/// A diacritic that modifies a letter without carrying a tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LetterMark {
    /// `â ê ô`
    Circumflex,
    /// `ă`
    Breve,
    /// `ơ ư`
    Horn,
    /// `đ`
    Stroke,
}

/// `(marked, base, mark)` for every letter the engine can build.
///
/// Uppercase entries are listed explicitly rather than derived, so that a missing
/// capital is a compile-time-visible hole in the table instead of a runtime case
/// fold.
const MARKED_LETTERS: &[(char, char, LetterMark)] = &[
    ('â', 'a', LetterMark::Circumflex),
    ('Â', 'A', LetterMark::Circumflex),
    ('ă', 'a', LetterMark::Breve),
    ('Ă', 'A', LetterMark::Breve),
    ('ê', 'e', LetterMark::Circumflex),
    ('Ê', 'E', LetterMark::Circumflex),
    ('ô', 'o', LetterMark::Circumflex),
    ('Ô', 'O', LetterMark::Circumflex),
    ('ơ', 'o', LetterMark::Horn),
    ('Ơ', 'O', LetterMark::Horn),
    ('ư', 'u', LetterMark::Horn),
    ('Ư', 'U', LetterMark::Horn),
    ('đ', 'd', LetterMark::Stroke),
    ('Đ', 'D', LetterMark::Stroke),
];

/// The mark `character` carries, or `None` when it is plain.
pub(crate) fn letter_mark(character: char) -> Option<LetterMark> {
    MARKED_LETTERS
        .iter()
        .find(|(marked, _, _)| *marked == character)
        .map(|(_, _, mark)| *mark)
}

/// The plain letter under `character`, or `character` itself when it is already
/// plain. Keeps the case, because callers restore a letter rather than normalize it.
pub(crate) fn base_letter(character: char) -> char {
    match MARKED_LETTERS
        .iter()
        .find(|(marked, _, _)| *marked == character)
    {
        Some((_, base, _)) => *base,
        None => character,
    }
}

/// Compose a plain `base` letter with `mark`, keeping `base`'s case.
///
/// Returns `None` when the pair is not a Vietnamese letter (`e` + breve) or when
/// `base` is already marked (`â` + circumflex); callers rely on the second case to
/// decide that a key must stay literal.
pub(crate) fn compose(base: char, mark: LetterMark) -> Option<char> {
    MARKED_LETTERS
        .iter()
        .find(|(_, candidate_base, candidate_mark)| {
            *candidate_base == base && *candidate_mark == mark
        })
        .map(|(marked, _, _)| *marked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_marked_letter_round_trips() {
        for (marked, base, mark) in MARKED_LETTERS {
            assert_eq!(base_letter(*marked), *base);
            assert_eq!(letter_mark(*marked), Some(*mark));
            assert_eq!(compose(*base, *mark), Some(*marked));
        }
        assert_eq!(letter_mark('a'), None);
        assert_eq!(letter_mark('ơ'), Some(LetterMark::Horn));
    }

    #[test]
    fn plain_letters_are_their_own_base() {
        for character in ['a', 'e', 'o', 'u', 'd', 'n', '1', '['] {
            assert_eq!(base_letter(character), character);
        }
    }

    #[test]
    fn impossible_pairs_have_no_letter() {
        // The table must not return a nearby letter for a pair Vietnamese never
        // spells, because callers read `None` as "keep this key literal".
        assert_eq!(compose('e', LetterMark::Breve), None);
        assert_eq!(compose('d', LetterMark::Horn), None);
        assert_eq!(compose('n', LetterMark::Circumflex), None);
        // A letter that already carries a mark is not a base letter.
        assert_eq!(compose('â', LetterMark::Circumflex), None);
    }

    #[test]
    fn composition_respects_case() {
        assert_eq!(compose('A', LetterMark::Circumflex), Some('Â'));
        assert_eq!(compose('D', LetterMark::Stroke), Some('Đ'));
        assert_eq!(compose('U', LetterMark::Horn), Some('Ư'));
    }
}
