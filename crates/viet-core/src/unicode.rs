//! The closed set of Vietnamese letters the engine can produce.
//!
//! The engine never calls a Unicode library: the characters a Vietnamese syllable
//! can contain are a small closed set, so the mapping is a static table (decision
//! D1 in `docs/SPEC-engine-core.md`). Every entry is checked against Unicode
//! canonical decomposition by `scripts/verify_unicode_table.py`.

/// A quality diacritic: it changes the vowel itself, not its tone.
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

/// A tone: the mark that says how the syllable sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tone {
    /// `s` in Telex: `á`
    Acute,
    /// `f`: `à`
    Grave,
    /// `r`: `ả`
    HookAbove,
    /// `x`: `ã`
    Tilde,
    /// `j`: `ạ`
    UnderDot,
}

impl Tone {
    /// Position of this tone in [`VowelForms::forms`].
    const fn index(self) -> usize {
        match self {
            Tone::Acute => 1,
            Tone::Grave => 2,
            Tone::HookAbove => 3,
            Tone::Tilde => 4,
            Tone::UnderDot => 5,
        }
    }
}

/// One vowel in all six spellings it can take.
struct VowelForms {
    base: char,
    quality: Option<LetterMark>,
    /// `[plain, acute, grave, hook above, tilde, under dot]`, indexed by
    /// [`Tone::index`].
    forms: [char; 6],
}

/// Every vowel the engine can write, lowercase rows first.
///
/// Uppercase rows are listed explicitly instead of being derived, so a missing
/// capital is a visible hole in the table rather than a case fold that silently
/// depends on Unicode data the engine does not ship.
const VOWEL_FORMS: &[VowelForms] = &[
    VowelForms {
        base: 'a',
        quality: None,
        forms: ['a', 'á', 'à', 'ả', 'ã', 'ạ'],
    },
    VowelForms {
        base: 'a',
        quality: Some(LetterMark::Breve),
        forms: ['ă', 'ắ', 'ằ', 'ẳ', 'ẵ', 'ặ'],
    },
    VowelForms {
        base: 'a',
        quality: Some(LetterMark::Circumflex),
        forms: ['â', 'ấ', 'ầ', 'ẩ', 'ẫ', 'ậ'],
    },
    VowelForms {
        base: 'e',
        quality: None,
        forms: ['e', 'é', 'è', 'ẻ', 'ẽ', 'ẹ'],
    },
    VowelForms {
        base: 'e',
        quality: Some(LetterMark::Circumflex),
        forms: ['ê', 'ế', 'ề', 'ể', 'ễ', 'ệ'],
    },
    VowelForms {
        base: 'i',
        quality: None,
        forms: ['i', 'í', 'ì', 'ỉ', 'ĩ', 'ị'],
    },
    VowelForms {
        base: 'o',
        quality: None,
        forms: ['o', 'ó', 'ò', 'ỏ', 'õ', 'ọ'],
    },
    VowelForms {
        base: 'o',
        quality: Some(LetterMark::Circumflex),
        forms: ['ô', 'ố', 'ồ', 'ổ', 'ỗ', 'ộ'],
    },
    VowelForms {
        base: 'o',
        quality: Some(LetterMark::Horn),
        forms: ['ơ', 'ớ', 'ờ', 'ở', 'ỡ', 'ợ'],
    },
    VowelForms {
        base: 'u',
        quality: None,
        forms: ['u', 'ú', 'ù', 'ủ', 'ũ', 'ụ'],
    },
    VowelForms {
        base: 'u',
        quality: Some(LetterMark::Horn),
        forms: ['ư', 'ứ', 'ừ', 'ử', 'ữ', 'ự'],
    },
    VowelForms {
        base: 'y',
        quality: None,
        forms: ['y', 'ý', 'ỳ', 'ỷ', 'ỹ', 'ỵ'],
    },
    VowelForms {
        base: 'A',
        quality: None,
        forms: ['A', 'Á', 'À', 'Ả', 'Ã', 'Ạ'],
    },
    VowelForms {
        base: 'A',
        quality: Some(LetterMark::Breve),
        forms: ['Ă', 'Ắ', 'Ằ', 'Ẳ', 'Ẵ', 'Ặ'],
    },
    VowelForms {
        base: 'A',
        quality: Some(LetterMark::Circumflex),
        forms: ['Â', 'Ấ', 'Ầ', 'Ẩ', 'Ẫ', 'Ậ'],
    },
    VowelForms {
        base: 'E',
        quality: None,
        forms: ['E', 'É', 'È', 'Ẻ', 'Ẽ', 'Ẹ'],
    },
    VowelForms {
        base: 'E',
        quality: Some(LetterMark::Circumflex),
        forms: ['Ê', 'Ế', 'Ề', 'Ể', 'Ễ', 'Ệ'],
    },
    VowelForms {
        base: 'I',
        quality: None,
        forms: ['I', 'Í', 'Ì', 'Ỉ', 'Ĩ', 'Ị'],
    },
    VowelForms {
        base: 'O',
        quality: None,
        forms: ['O', 'Ó', 'Ò', 'Ỏ', 'Õ', 'Ọ'],
    },
    VowelForms {
        base: 'O',
        quality: Some(LetterMark::Circumflex),
        forms: ['Ô', 'Ố', 'Ồ', 'Ổ', 'Ỗ', 'Ộ'],
    },
    VowelForms {
        base: 'O',
        quality: Some(LetterMark::Horn),
        forms: ['Ơ', 'Ớ', 'Ờ', 'Ở', 'Ỡ', 'Ợ'],
    },
    VowelForms {
        base: 'U',
        quality: None,
        forms: ['U', 'Ú', 'Ù', 'Ủ', 'Ũ', 'Ụ'],
    },
    VowelForms {
        base: 'U',
        quality: Some(LetterMark::Horn),
        forms: ['Ư', 'Ứ', 'Ừ', 'Ử', 'Ữ', 'Ự'],
    },
    VowelForms {
        base: 'Y',
        quality: None,
        forms: ['Y', 'Ý', 'Ỳ', 'Ỷ', 'Ỹ', 'Ỵ'],
    },
];

/// The row that contains `character` in any of its spellings.
fn vowel_forms(character: char) -> Option<&'static VowelForms> {
    VOWEL_FORMS
        .iter()
        .find(|row| row.forms.contains(&character))
}

/// The quality mark `character` carries, or `None` when it carries none.
///
/// A tone is not a quality mark: `letter_mark('ế')` is `Circumflex`, because tone
/// placement cares what changed the vowel, not how it sounds.
pub(crate) fn letter_mark(character: char) -> Option<LetterMark> {
    if let Some(row) = vowel_forms(character) {
        return row.quality;
    }
    match character {
        'đ' | 'Đ' => Some(LetterMark::Stroke),
        _ => None,
    }
}

/// The plain letter under `character`, or `character` itself when it is already
/// plain. Keeps the case, because callers restore a letter rather than normalize it.
pub(crate) fn base_letter(character: char) -> char {
    if let Some(row) = vowel_forms(character) {
        return row.base;
    }
    match character {
        'đ' => 'd',
        'Đ' => 'D',
        _ => character,
    }
}

/// Compose a plain `base` letter with `mark`, keeping `base`'s case.
///
/// Returns `None` when the pair is not a Vietnamese letter (`e` + breve) or when
/// `base` is already marked (`â` + circumflex); callers rely on the second case to
/// decide that a key must stay literal.
pub(crate) fn compose(base: char, mark: LetterMark) -> Option<char> {
    if mark == LetterMark::Stroke {
        return match base {
            'd' => Some('đ'),
            'D' => Some('Đ'),
            _ => None,
        };
    }
    VOWEL_FORMS
        .iter()
        .find(|row| row.base == base && row.quality == Some(mark))
        .map(|row| row.forms[0])
}

/// Put `tone` on `character`, keeping its quality mark and case.
///
/// Returns `None` when the character cannot carry a tone at all.
pub(crate) fn with_tone(character: char, tone: Tone) -> Option<char> {
    vowel_forms(character).map(|row| row.forms[tone.index()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_form_belongs_to_exactly_one_row() {
        for (index, row) in VOWEL_FORMS.iter().enumerate() {
            for character in row.forms {
                let owner = vowel_forms(character).expect("in the table");
                assert_eq!(
                    owner.forms, row.forms,
                    "{character:?} is listed in more than one row"
                );
            }
            assert_eq!(
                index,
                VOWEL_FORMS
                    .iter()
                    .position(|r| r.forms == row.forms)
                    .unwrap()
            );
        }
    }

    #[test]
    fn every_row_has_six_distinct_spellings() {
        for row in VOWEL_FORMS {
            let mut forms = row.forms;
            forms.sort_unstable();
            for pair in forms.windows(2) {
                assert_ne!(pair[0], pair[1], "duplicate spelling in {:?}", row.forms);
            }
        }
    }

    #[test]
    fn uppercase_rows_mirror_the_lowercase_ones() {
        for row in VOWEL_FORMS
            .iter()
            .filter(|row| row.base.is_ascii_lowercase())
        {
            let uppercase_base = row.base.to_ascii_uppercase();
            let upper = VOWEL_FORMS
                .iter()
                .find(|candidate| {
                    candidate.base == uppercase_base && candidate.quality == row.quality
                })
                .unwrap_or_else(|| panic!("no uppercase row for {:?}", row.forms));
            for (lower, upper) in row.forms.iter().zip(upper.forms.iter()) {
                let mut expected = lower.to_uppercase();
                assert_eq!(Some(*upper), expected.next(), "{lower:?} -> {upper:?}");
            }
        }
    }

    #[test]
    fn base_and_quality_round_trip() {
        for row in VOWEL_FORMS {
            for character in row.forms {
                assert_eq!(base_letter(character), row.base);
                assert_eq!(letter_mark(character), row.quality);
            }
            match row.quality {
                Some(quality) => assert_eq!(compose(row.base, quality), Some(row.forms[0])),
                // The stroke is the only mark a plain vowel cannot take, and it is
                // not a vowel mark at all: `compose` reads it as "make this letter a
                // `đ`", which a vowel never becomes.
                None => assert_eq!(compose(row.base, LetterMark::Stroke), None),
            }
        }
        assert_eq!(base_letter('đ'), 'd');
        assert_eq!(base_letter('Đ'), 'D');
        assert_eq!(letter_mark('đ'), Some(LetterMark::Stroke));
        assert_eq!(compose('d', LetterMark::Stroke), Some('đ'));
        assert_eq!(compose('D', LetterMark::Stroke), Some('Đ'));
    }

    #[test]
    fn tones_replace_each_other_without_touching_the_quality() {
        assert_eq!(with_tone('a', Tone::Acute), Some('á'));
        assert_eq!(with_tone('ă', Tone::UnderDot), Some('ặ'));
        assert_eq!(with_tone('â', Tone::Tilde), Some('ẫ'));
        assert_eq!(with_tone('ế', Tone::Grave), Some('ề'));
        assert_eq!(with_tone('Ư', Tone::HookAbove), Some('Ử'));
        assert_eq!(with_tone('n', Tone::Acute), None);
    }

    #[test]
    fn impossible_pairs_have_no_letter() {
        // The table must not return a nearby letter for a pair Vietnamese never
        // spells, because callers read `None` as "keep this key literal".
        assert_eq!(compose('e', LetterMark::Breve), None);
        assert_eq!(compose('d', LetterMark::Horn), None);
        assert_eq!(compose('n', LetterMark::Circumflex), None);
        assert_eq!(compose('â', LetterMark::Circumflex), None);
    }
}
