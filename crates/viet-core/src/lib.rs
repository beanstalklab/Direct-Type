//! Pure transformation engine.
//!
//! Turns key events into an [`Action`] describing what an input-method frontend
//! should do. This crate must never depend on fcitx5, GTK, Qt, X11, Wayland, the
//! filesystem, or the environment. See `docs/SPEC-engine-core.md`.
//!
//! Status: Telex letters and tone keys work (T1.2, T1.4), and the two validity
//! switches work (T1.5): `auto_restore` gives back the raw keystrokes of a word that
//! is not Vietnamese, `spell_check` refuses to transform a word that can no longer
//! become one. Characters are committed immediately and never held in a preedit
//! buffer, so a modifier rewrites what the application already shows instead of
//! building a composition string. VNI (T1.7) and Simple Telex's restricted modifier
//! set (T1.8) are not implemented yet.

#![forbid(unsafe_code)]

mod input;
mod spelling;
mod syllable;
mod telex;
mod tone;
mod unicode;
mod vni;

/// Input method used to interpret keystrokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Method {
    /// Full Telex: tone keys, `w`/`aa`/`ee`/`oo`, `dd`, `[`/`]`.
    #[default]
    Telex,
    /// Restricted Telex: only `aa`/`ee`/`oo`, `w`, `dd`.
    SimpleTelex,
    /// VNI: digits `1..5` for tones, `6..9` for letter modifiers.
    Vni,
}

/// Tone placement convention for `oa` / `oe` / `uy` open syllables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToneStyle {
    /// `hoà`, `thuỷ`, `tuỳ`
    Old,
    /// `hòa`, `thủy`, `túy`
    #[default]
    New,
}

/// Engine configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub method: Method,
    pub tone_style: ToneStyle,
    /// Give back the raw keystrokes when the finished word is not a Vietnamese
    /// syllable. Applied at the word boundary, where "not Vietnamese" becomes true.
    pub auto_restore: bool,
    /// Never transform a word that can no longer become a Vietnamese syllable.
    /// Stricter than `auto_restore`: the transformation is refused while typing.
    pub spell_check: bool,
    /// Accept a tone key typed at any position and reposition the mark.
    pub free_tone_mark: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            method: Method::default(),
            tone_style: ToneStyle::default(),
            auto_restore: true,
            spell_check: false,
            free_tone_mark: true,
        }
    }
}

/// How many raw keys one word may hold.
///
/// A Vietnamese syllable is at most seven letters, and a rule pack at most doubles that,
/// so nothing a person types comes close. The cap is there so a long run of keys without
/// a boundary — a pasted token, a stuck key — cannot grow the buffer without end. A word
/// that reaches it ends as it stands, and the next key starts a new one.
const MAXIMUM_WORD_KEYS: usize = 64;

/// One key event handed to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Printable character after the frontend applied shift and layout mapping.
    Char(char),
    Backspace,
    /// Space, tab, enter, punctuation, or focus loss: ends the current word and is
    /// the moment `auto_restore` decides whether the word has to be given back.
    Boundary,
    /// Drop all state (focus change, language switch).
    Reset,
}

/// What the frontend should do with the key that produced this action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action<'a> {
    /// The frontend must not forward the original key.
    pub consumed: bool,
    /// Characters to delete before committing.
    pub backspaces: u16,
    /// Text the application should end up with.
    pub commit: &'a str,
    /// Text to show while composing; always empty in the direct-typing path.
    pub preedit: &'a str,
    /// A word just ended on this input.
    pub boundary: bool,
}

impl Action<'static> {
    /// A no-op action: forward the key untouched.
    pub const fn none() -> Self {
        Self {
            consumed: false,
            backspaces: 0,
            commit: "",
            preedit: "",
            boundary: false,
        }
    }
}

/// Transformation state machine.
#[derive(Debug, Clone)]
pub struct Engine {
    config: Config,
    /// Keystrokes of the word currently being typed, exactly as typed. The
    /// transformation is re-derived from this on every key, which is what makes
    /// backspace (T2.1) and auto-restore (T1.5) exact rather than incremental.
    raw_keys: String,
    /// Text the application currently shows for this word: the previous
    /// transformation. Kept only to report the smallest delete-and-replace.
    shown_text: String,
    /// Reused buffer for the next transformation, so the hot path does not
    /// allocate once warmed up.
    scratch: String,
}

impl Engine {
    pub const fn new(config: Config) -> Self {
        Self {
            config,
            raw_keys: String::new(),
            shown_text: String::new(),
            scratch: String::new(),
        }
    }

    /// Feed one key event and return what the frontend should perform.
    pub fn feed(&mut self, input: Input) -> Action<'_> {
        match input {
            Input::Char(character) => {
                if self.raw_keys.chars().count() >= MAXIMUM_WORD_KEYS {
                    return self.start_new_word(character);
                }
                self.raw_keys.push(character);
                self.rewrite()
            }
            Input::Backspace => {
                // Deleting a keystroke can change earlier characters, so the word is
                // re-derived instead of only losing its last character.
                if self.raw_keys.pop().is_none() {
                    return Action::none();
                }
                self.rewrite()
            }
            Input::Boundary => {
                if self.raw_keys.is_empty() {
                    self.clear_word();
                    return Action::none();
                }
                // The word is over, so this is the first moment the engine knows
                // whether the user typed Vietnamese or something that only looked
                // like it while the marks were still being added.
                if self.config.auto_restore
                    && spelling::restore_is_needed(&self.shown_text, &self.raw_keys)
                {
                    return self.restore_raw_keys();
                }
                self.clear_word();
                Action {
                    consumed: false,
                    backspaces: 0,
                    commit: "",
                    preedit: "",
                    boundary: true,
                }
            }
            Input::Reset => {
                self.clear_word();
                Action::none()
            }
        }
    }

    /// Re-derive the word and report the smallest edit that reaches it.
    fn rewrite(&mut self) -> Action<'_> {
        self.transform_into_scratch();

        // Characters, not bytes: the ABI counts deletions in characters.
        let (backspaces, shared_bytes) = difference(&self.shown_text, &self.scratch);

        // The freshly derived text becomes the shown one; the old text is recycled
        // as the next scratch buffer.
        core::mem::swap(&mut self.shown_text, &mut self.scratch);

        Action {
            // The key is never forwarded: the frontend deletes `backspaces`
            // characters and inserts `commit`. That is what keeps a composition
            // buffer out of the application, so nothing is rendered underlined.
            consumed: true,
            backspaces,
            commit: &self.shown_text[shared_bytes..],
            preedit: "",
            boundary: false,
        }
    }

    /// End the word that reached [`MAXIMUM_WORD_KEYS`] and start the next one with
    /// `key`.
    ///
    /// The over-long word is not given back the way `auto_restore` gives back a finished
    /// word: it never reached a boundary, and no person types one. Its text stays exactly
    /// as the application already shows it, so no character is lost, and the buffer is
    /// free again.
    fn start_new_word(&mut self, key: char) -> Action<'_> {
        self.raw_keys.clear();
        self.shown_text.clear();
        self.raw_keys.push(key);
        self.rewrite()
    }

    /// Give back the keystrokes of the word that just ended.
    ///
    /// The frontend deletes the transformed text and inserts the raw keys, which is
    /// the only way to undo marks that were already committed. The boundary key
    /// itself is not consumed, so a space still reaches the application afterwards.
    fn restore_raw_keys(&mut self) -> Action<'_> {
        let (backspaces, shared_bytes) = difference(&self.shown_text, &self.raw_keys);

        // The restored word is what the application will show, and the word it
        // belongs to is over, so the engine's own view of the shown text starts empty
        // again. The commit therefore points at the recycle buffer: keeping the word
        // in `shown_text` would make the next word report deletions against text the
        // frontend has already replaced.
        self.scratch.clear();
        self.scratch.push_str(&self.raw_keys);
        self.shown_text.clear();
        self.raw_keys.clear();

        Action {
            consumed: false,
            backspaces,
            commit: &self.scratch[shared_bytes..],
            preedit: "",
            boundary: true,
        }
    }

    /// Put the transformation of the current word into `scratch`.
    fn transform_into_scratch(&mut self) {
        match self.config.method {
            Method::Telex => {
                input::apply::<telex::Telex>(&self.raw_keys, &self.config, &mut self.scratch)
            }
            Method::Vni => {
                input::apply::<vni::Vni>(&self.raw_keys, &self.config, &mut self.scratch)
            }
            Method::SimpleTelex => {
                input::apply::<telex::SimpleTelex>(&self.raw_keys, &self.config, &mut self.scratch)
            }
        }
        // `spell_check` refuses a transformation the word can no longer justify, so
        // `asd` keeps its `s` instead of showing `ád` and losing it at the boundary.
        // The raw keys are the same text for a word that was never transformed.
        if self.config.spell_check && !spelling::can_become_valid_syllable(&self.scratch) {
            self.scratch.clear();
            self.scratch.push_str(&self.raw_keys);
        }
    }

    fn clear_word(&mut self) {
        self.raw_keys.clear();
        self.shown_text.clear();
        self.scratch.clear();
    }

    /// Drop all state, for example when focus or language changes.
    pub fn reset(&mut self) {
        self.clear_word();
    }

    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Replace the configuration.
    ///
    /// The caller must [`reset`](Self::reset) when the input method changes: the
    /// shown word was derived with the old rule pack, so continuing it would report
    /// deltas against text the new pack would never produce.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }
}

/// Characters and byte length shared by the start of both texts.
fn shared_prefix(left: &str, right: &str) -> (usize, usize) {
    let mut characters = 0;
    let mut bytes = 0;
    let mut right_characters = right.chars();
    for left_character in left.chars() {
        let Some(right_character) = right_characters.next() else {
            break;
        };
        if left_character != right_character {
            break;
        }
        characters += 1;
        bytes += left_character.len_utf8();
    }
    (characters, bytes)
}

/// The edit that turns `from` into `to`: how many characters to delete and the byte
/// offset in `to` where the insertion starts.
fn difference(from: &str, to: &str) -> (u16, usize) {
    let (shared_characters, shared_bytes) = shared_prefix(from, to);
    // Characters, not bytes: the ABI counts deletions in characters. The clamp is
    // unreachable for a real word; it keeps the 16-bit field honest instead of
    // wrapping to a small number that would corrupt the word.
    let backspaces = u16::try_from(from.chars().count() - shared_characters).unwrap_or(u16::MAX);
    (backspaces, shared_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        Engine::new(Config::default())
    }

    /// Feed `keys` and return the text an application would show afterwards.
    fn typed_text(engine: &mut Engine, keys: &str) -> String {
        let mut text = String::new();
        for character in keys.chars() {
            perform(&mut text, engine.feed(Input::Char(character)));
        }
        text
    }

    /// Feed `keys`, end the word, and return the text an application would show.
    ///
    /// `auto_restore` only acts at the boundary, so a test that stops at the last
    /// character never sees it.
    fn finished_text(engine: &mut Engine, keys: &str) -> String {
        let mut text = typed_text(engine, keys);
        perform(&mut text, engine.feed(Input::Boundary));
        text
    }

    /// Make `text` look like the application after performing `action`.
    fn perform(text: &mut String, action: Action<'_>) {
        for _ in 0..action.backspaces {
            text.pop();
        }
        text.push_str(action.commit);
    }

    fn finished_text_with(auto_restore: bool, spell_check: bool, keys: &str) -> String {
        let mut engine = Engine::new(Config {
            auto_restore,
            spell_check,
            ..Config::default()
        });
        finished_text(&mut engine, keys)
    }

    #[test]
    fn character_is_committed_immediately() {
        let mut engine = engine();
        let action = engine.feed(Input::Char('a'));
        assert!(action.consumed);
        assert_eq!(action.backspaces, 0);
        assert_eq!(action.commit, "a");
        assert_eq!(action.preedit, "");
        assert!(!action.boundary);
    }

    #[test]
    fn characters_accumulate_in_the_commit_text() {
        let mut engine = engine();
        let mut committed = String::new();
        for character in "abc".chars() {
            committed.push_str(engine.feed(Input::Char(character)).commit);
        }
        assert_eq!(committed, "abc");
    }

    #[test]
    fn modifier_replaces_the_character_already_committed() {
        let mut engine = engine();
        engine.feed(Input::Char('a'));
        let action = engine.feed(Input::Char('a'));
        assert!(action.consumed);
        assert_eq!(action.backspaces, 1);
        assert_eq!(action.commit, "â");
    }

    #[test]
    fn typed_words_match_telex() {
        let cases = [
            ("viee", "viê"),
            ("ddaay", "đây"),
            ("thuowng", "thương"),
            ("aa", "â"),
            ("aaa", "aa"),
            ("dd", "đ"),
        ];
        for (raw, expected) in cases {
            assert_eq!(typed_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn tone_key_replaces_the_vowel_already_committed() {
        let mut engine = engine();
        engine.feed(Input::Char('a'));
        let action = engine.feed(Input::Char('s'));
        assert!(action.consumed);
        assert_eq!(action.backspaces, 1);
        assert_eq!(action.commit, "á");
    }

    #[test]
    fn tone_keys_mark_the_vowel_the_syllable_rules_choose() {
        let cases = [
            ("as", "á"),
            ("vieejt", "việt"),
            ("toans", "toán"),
            ("tieengs", "tiếng"),
            ("dduowngf", "đường"),
            ("quyeenf", "quyền"),
            ("chuyeenf", "chuyền"),
            ("nguyeenx", "nguyễn"),
        ];
        for (raw, expected) in cases {
            assert_eq!(typed_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn a_later_tone_key_replaces_the_tone() {
        let cases = [
            ("af", "à"),
            ("asf", "à"),
            ("asfr", "ả"),
            ("asfrx", "ã"),
            ("asfrxj", "ạ"),
        ];
        for (raw, expected) in cases {
            assert_eq!(typed_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn doubling_a_tone_key_removes_the_tone_and_keeps_the_letter() {
        let cases = [
            ("ass", "as"),
            ("aass", "âs"),
            ("toanss", "toans"),
            // Shift is not a different key: the second press undoes the mark, and the
            // letter that comes back is the one the pending tone was typed with.
            ("ASs", "AS"),
            ("AsS", "As"),
        ];
        for (raw, expected) in cases {
            assert_eq!(typed_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn z_removes_the_tone_and_keeps_the_tone_key_as_a_letter() {
        // The tone key returns to the position where it was typed, not to the end,
        // so a removal in the middle of a word restores the original spelling.
        let cases = [("asz", "as"), ("asdz", "asd"), ("toansz", "toans")];
        for (raw, expected) in cases {
            assert_eq!(typed_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn free_tone_mark_accepts_the_mark_before_the_coda() {
        // With the default setting the mark may be typed as soon as a vowel exists;
        // adding the coda afterwards moves it to the position the rules dictate.
        let cases = [
            ("vieejt", "việt"),
            ("vieetj", "việt"),
            ("toasn", "toán"),
            ("hoasn", "hoán"),
        ];
        for (raw, expected) in cases {
            assert_eq!(typed_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn without_free_tone_mark_the_mark_must_follow_the_last_letter() {
        let mut engine = Engine::new(Config {
            free_tone_mark: false,
            ..Config::default()
        });
        // `j` before the final `t` stays a letter, because the mark has to come last.
        assert_eq!(typed_text(&mut engine, "vieejt"), "viêjt");
        let mut engine = Engine::new(Config {
            free_tone_mark: false,
            ..Config::default()
        });
        assert_eq!(typed_text(&mut engine, "vieetj"), "việt");
        let mut engine = Engine::new(Config {
            free_tone_mark: false,
            ..Config::default()
        });
        assert_eq!(typed_text(&mut engine, "toans"), "toán");
    }

    #[test]
    fn tone_style_chooses_the_placement_of_oa_oe_uy() {
        let cases = [
            (ToneStyle::New, "hoaf", "hòa"),
            (ToneStyle::New, "thuyr", "thủy"),
            (ToneStyle::Old, "hoaf", "hoà"),
            (ToneStyle::Old, "thuyr", "thuỷ"),
        ];
        for (tone_style, raw, expected) in cases {
            let mut engine = Engine::new(Config {
                tone_style,
                ..Config::default()
            });
            assert_eq!(
                typed_text(&mut engine, raw),
                expected,
                "raw {raw:?} with {tone_style:?}"
            );
        }
        // Counter-examples: every other open pair keeps the first vowel.
        for tone_style in [ToneStyle::New, ToneStyle::Old] {
            for (raw, expected) in [("cais", "cái"), ("cuar", "của")] {
                let mut engine = Engine::new(Config {
                    tone_style,
                    ..Config::default()
                });
                assert_eq!(
                    typed_text(&mut engine, raw),
                    expected,
                    "raw {raw:?} with {tone_style:?}"
                );
            }
        }
    }

    #[test]
    fn a_tone_key_without_a_vowel_is_a_letter() {
        // The tone keys are letters of the alphabet: outside a syllable they must
        // reach the application unchanged, and a word that never becomes one
        // syllable keeps every keystroke.
        let cases = [
            ("sz", "sz"),
            ("z", "z"),
            ("test", "tét"),
            ("telex", "telex"),
            ("ashello", "ashello"),
        ];
        for (raw, expected) in cases {
            assert_eq!(typed_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn tone_key_case_follows_the_vowel_carrying_the_mark() {
        let cases = [
            ("AS", "Á"),
            ("As", "Á"),
            ("aS", "á"),
            ("VIEEJT", "VIỆT"),
            ("Vieejt", "Việt"),
        ];
        for (raw, expected) in cases {
            assert_eq!(typed_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn boundary_ends_the_word_and_clears_the_transformation() {
        let mut engine = engine();
        assert_eq!(typed_text(&mut engine, "aa"), "â");
        let action = engine.feed(Input::Boundary);
        assert!(action.boundary);
        assert!(!action.consumed);
        // The next word starts from its own keystrokes, not from `â`.
        assert_eq!(typed_text(&mut engine, "a"), "a");
    }

    #[test]
    fn boundary_on_empty_buffer_does_not_report_a_word() {
        let mut engine = engine();
        let action = engine.feed(Input::Boundary);
        assert!(!action.boundary);
    }

    #[test]
    fn auto_restore_gives_back_the_keystrokes_of_an_impossible_word() {
        let cases = [
            // A coda Vietnamese never spells.
            ("asd", "asd"),
            ("asdf", "asdf"),
            // A modifier that produced a letter no syllable contains.
            ("oow", "oow"),
            // An undone tone that left a mark no syllable ends with.
            ("aass", "aass"),
            // Words that are Vietnamese keep their marks.
            ("dduowngf", "đường"),
            ("vieejt", "việt"),
            ("aw", "ă"),
        ];
        for (raw, expected) in cases {
            assert_eq!(finished_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
    }

    #[test]
    fn auto_restore_leaves_an_undone_modifier_alone() {
        // Nothing to undo: what the user sees is what they typed, minus the key that
        // undid the mark.
        let cases = [("aaa", "aa"), ("ddd", "dd"), ("aww", "aw"), ("uoww", "uow")];
        for (raw, expected) in cases {
            assert_eq!(finished_text(&mut engine(), raw), expected, "raw {raw:?}");
        }
        // A letter with no vowel is not a mangled word, so `đ` survives too.
        assert_eq!(finished_text(&mut engine(), "dd"), "đ");
    }

    #[test]
    fn auto_restore_cannot_tell_an_english_word_from_a_vietnamese_one() {
        // `tét` is real Vietnamese — `nét`, `mét` and `sét` share its rhyme — so a
        // rule that only asks "is this a syllable?" has to keep it. Typing English
        // needs the language toggle, or `free_tone_mark` off, which leaves the `s`
        // literal because it no longer trails the word.
        assert_eq!(finished_text(&mut engine(), "test"), "tét");
        let mut engine = Engine::new(Config {
            free_tone_mark: false,
            ..Config::default()
        });
        assert_eq!(finished_text(&mut engine, "test"), "test");
    }

    #[test]
    fn the_two_validity_switches_are_independent() {
        // keys, neither switch, auto_restore only, spell_check only, both.
        let cases = [
            ("asd", "ád", "asd", "asd", "asd"),
            ("aaa", "aa", "aa", "aaa", "aaa"),
            ("test", "tét", "tét", "tét", "tét"),
            ("hello", "hello", "hello", "hello", "hello"),
            ("aw", "ă", "ă", "ă", "ă"),
            ("vieejt", "việt", "việt", "việt", "việt"),
        ];
        for (raw, neither, restore_only, spell_check_only, both) in cases {
            assert_eq!(
                finished_text_with(false, false, raw),
                neither,
                "raw {raw:?} with neither switch"
            );
            assert_eq!(
                finished_text_with(true, false, raw),
                restore_only,
                "raw {raw:?} with auto_restore"
            );
            assert_eq!(
                finished_text_with(false, true, raw),
                spell_check_only,
                "raw {raw:?} with spell_check"
            );
            assert_eq!(
                finished_text_with(true, true, raw),
                both,
                "raw {raw:?} with both switches"
            );
        }
    }

    #[test]
    fn a_restore_reports_the_whole_word_for_replacement() {
        let mut engine = engine();
        assert_eq!(typed_text(&mut engine, "asd"), "ád");
        let action = engine.feed(Input::Boundary);
        // The space still has to reach the application: the frontend deletes the
        // word, inserts the keystrokes, and then forwards the key it was given.
        assert!(!action.consumed);
        assert!(action.boundary);
        assert_eq!(action.backspaces, 2);
        assert_eq!(action.commit, "asd");
        assert_eq!(action.preedit, "");
        // The restored word leaves no state behind for the next one.
        assert_eq!(finished_text(&mut engine, "as"), "á");
    }

    #[test]
    fn a_restored_word_leaves_nothing_behind_for_the_next_one() {
        let mut engine = engine();
        assert_eq!(finished_text(&mut engine, "asd"), "asd");
        // The frontend already replaced the restored word, so the first keystroke of
        // the next word must not delete anything.
        let action = engine.feed(Input::Char('t'));
        assert_eq!(action.backspaces, 0);
        assert_eq!(action.commit, "t");
        assert_eq!(typed_text(&mut engine, "oan"), "oan");
    }

    #[test]
    fn without_auto_restore_the_best_transformation_survives_the_boundary() {
        assert_eq!(finished_text_with(false, false, "asd"), "ád");
        assert_eq!(finished_text_with(false, false, "asdz"), "asd");
        // The T1.4 undo rules, which `auto_restore` on rewrites at the boundary.
        assert_eq!(finished_text_with(false, false, "aass"), "âs");
    }

    #[test]
    fn spell_check_refuses_a_transformation_the_word_cannot_justify() {
        let mut engine = Engine::new(Config {
            spell_check: true,
            ..Config::default()
        });
        // `aa` is not a syllable and can become none, so the third `a` is left as typed.
        assert_eq!(typed_text(&mut engine, "aaa"), "aaa");

        // One keystroke at a time, following the text the application would show: the
        // middle of `uwow` is the spelling `spell_check` gives up on, and the next
        // keystroke brings the transformation back.
        let mut engine = Engine::new(Config {
            spell_check: true,
            ..Config::default()
        });
        let mut text = String::new();
        for key in "uwo".chars() {
            perform(&mut text, engine.feed(Input::Char(key)));
        }
        assert_eq!(text, "uwo");
        perform(&mut text, engine.feed(Input::Char('w')));
        assert_eq!(text, "ươ");

        // Without the switch the same keys show the transformation they reach.
        let mut engine = Engine::new(Config::default());
        let mut text = String::new();
        for key in "uwo".chars() {
            perform(&mut text, engine.feed(Input::Char(key)));
        }
        assert_eq!(text, "ưo");
        perform(&mut text, engine.feed(Input::Char('w')));
        assert_eq!(text, "ươ");
    }

    /// A deterministic keystroke stream, so a failing word can be typed again
    /// without adding a random number dependency.
    struct Keys(u64);

    impl Keys {
        const ALPHABET: [char; 25] = [
            'a', 'e', 'i', 'o', 'u', 'y', 'd', 'w', 's', 'f', 'r', 'x', 'j', 'z', 'n', 'g', 'h',
            't', 'c', 'q', 'p', 'l', 'm', '1', '[',
        ];

        fn next_number(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }

        fn next_character(&mut self) -> char {
            Self::ALPHABET[self.next_number() as usize % Self::ALPHABET.len()]
        }

        fn next_word(&mut self) -> String {
            let length = self.next_number() as usize % 8 + 1;
            (0..length).map(|_| self.next_character()).collect()
        }
    }

    #[test]
    fn output_never_contains_a_combining_mark() {
        // Every character the engine inserts comes from the closed table in
        // `unicode.rs`, which is precomposed, so a word typed without combining marks
        // has to come out precomposed — the NFC guarantee in spec §2.1, checked here
        // without a Unicode dependency.
        let mut keys = Keys(0xfeed);
        for _ in 0..500 {
            let word = keys.next_word();
            let text = finished_text_with(false, false, &word);
            for character in text.chars() {
                assert!(
                    !('\u{0300}'..='\u{036F}').contains(&character),
                    "{word:?} produced the combining mark U+{:04X}",
                    u32::from(character)
                );
            }
        }
    }

    #[test]
    fn a_restore_never_loses_or_reorders_a_keystroke() {
        let mut keys = Keys(0x5eed);
        for _ in 0..2_000 {
            let word = keys.next_word();
            let restored = finished_text_with(true, false, &word);
            let transformed = finished_text_with(false, false, &word);
            if restored != transformed {
                assert_eq!(
                    restored, word,
                    "the restore of {word:?} must be exactly its keystrokes"
                );
            }
            assert!(
                restored == word || restored == transformed,
                "{word:?} shows a mixture of the keystrokes and the transformation"
            );
        }
    }

    #[test]
    fn backspace_rederives_the_word() {
        let mut engine = engine();
        assert_eq!(typed_text(&mut engine, "viee"), "viê");

        let action = engine.feed(Input::Backspace);
        assert!(action.consumed);
        // `viê` minus the last keystroke is `vie`, so the mark has to go too.
        assert_eq!(action.backspaces, 1);
        assert_eq!(action.commit, "e");
        assert!(!action.boundary);
    }

    #[test]
    fn a_word_that_reaches_the_key_cap_ends_and_the_next_key_starts_over() {
        let mut engine = engine();
        let mut text = String::new();
        for _ in 0..MAXIMUM_WORD_KEYS {
            perform(&mut text, engine.feed(Input::Char('a')));
        }
        assert_eq!(engine.raw_keys.chars().count(), MAXIMUM_WORD_KEYS);
        let characters_before = text.chars().count();

        // The cap ends the word where it stands and starts a new one with this key, in
        // one action: nothing on screen is deleted and the new key is committed.
        let action = engine.feed(Input::Char('a'));
        let (backspaces, commit) = (action.backspaces, action.commit);
        text.push_str(commit);
        assert_eq!(backspaces, 0);
        assert_eq!(text.chars().count(), characters_before + 1);
        assert_eq!(engine.raw_keys, "a");

        // The buffer stays bounded however long the run of keys is. The text grows by
        // less than a key per keystroke because a pair of `a`s becomes one `â`, so the
        // count is not asserted here; the cap test above is the one about losing text.
        for _ in 0..1_000 {
            perform(&mut text, engine.feed(Input::Char('a')));
            assert!(engine.raw_keys.chars().count() <= MAXIMUM_WORD_KEYS);
        }
    }

    #[test]
    fn backspace_on_an_empty_word_is_forwarded() {
        let mut engine = engine();
        let action = engine.feed(Input::Backspace);
        assert!(!action.consumed);
        assert_eq!(action, Action::none());
    }

    #[test]
    fn backspace_after_a_boundary_does_not_recall_the_word() {
        let mut engine = engine();
        assert_eq!(finished_text(&mut engine, "vieejt"), "việt");
        // The word is over, so the key belongs to the application: it deletes whatever
        // came before the next word, and the engine must not rewrite that text.
        let action = engine.feed(Input::Backspace);
        assert!(!action.consumed);
        assert_eq!(action, Action::none());
        assert_eq!(typed_text(&mut engine, "a"), "a");
    }

    #[test]
    fn backspace_after_a_restored_word_does_not_recall_it() {
        let mut engine = engine();
        assert_eq!(finished_text(&mut engine, "asd"), "asd");
        assert_eq!(engine.feed(Input::Backspace), Action::none());
        assert_eq!(typed_text(&mut engine, "a"), "a");
    }

    #[test]
    fn backspace_removes_a_keystroke_not_a_transformed_character() {
        // Deleting the `s` of `aas` takes the tone off `ấ` and leaves `â`, because the
        // raw keys are `aa` again. A backspace that deleted a character of the shown
        // text would leave `ấ` minus its last letter.
        let mut engine = engine();
        assert_eq!(typed_text(&mut engine, "aas"), "ấ");
        let action = engine.feed(Input::Backspace);
        assert!(action.consumed);
        assert_eq!(action.backspaces, 1);
        assert_eq!(action.commit, "â");
    }

    #[test]
    fn backspace_can_bring_a_removed_tone_back() {
        // `asz` removes the tone and keeps the `s`; deleting that `s` is `as`, which is
        // `á` again.
        let mut engine = engine();
        assert_eq!(typed_text(&mut engine, "asz"), "as");
        let action = engine.feed(Input::Backspace);
        assert_eq!(action.backspaces, 2);
        assert_eq!(action.commit, "á");
    }

    #[test]
    fn backspace_undoes_a_modifier_that_shaped_earlier_letters() {
        // `dduowng` is `đương`; the `w` horned two letters, so deleting the `g` has to
        // re-derive from `dduowng` minus `g` rather than edit the shown text. A tone
        // typed afterwards lands on the shorter word.
        let mut engine = engine();
        let mut text = String::new();
        for key in "dduowng".chars() {
            perform(&mut text, engine.feed(Input::Char(key)));
        }
        assert_eq!(text, "đương");
        perform(&mut text, engine.feed(Input::Backspace));
        assert_eq!(text, "đươn");
        perform(&mut text, engine.feed(Input::Char('f')));
        assert_eq!(text, "đườn");
    }

    #[test]
    fn backspace_re_derives_under_spell_check_too() {
        // `aaa` cannot become a syllable, so `spell_check` leaves it as typed. One
        // delete later the word is `aa`, which is a syllable again, and the engine
        // transforms it without being asked twice.
        let mut engine = Engine::new(Config {
            spell_check: true,
            ..Config::default()
        });
        let mut text = String::new();
        for key in "aaa".chars() {
            perform(&mut text, engine.feed(Input::Char(key)));
        }
        assert_eq!(text, "aaa");
        perform(&mut text, engine.feed(Input::Backspace));
        assert_eq!(text, "â");
    }

    #[test]
    fn backspaces_leave_the_state_of_the_surviving_keys() {
        // A word typed with deletes in the middle must end up exactly where the
        // surviving keystrokes would have ended, and the raw buffer must say so.
        let mut keys = Keys(0xbadc0de);
        for _ in 0..500 {
            let mut engine = engine();
            let mut typed = String::new();
            let mut surviving = String::new();
            let mut text = String::new();
            for _ in 0..keys.next_number() as usize % 10 + 1 {
                if keys.next_number() % 3 == 0 {
                    perform(&mut text, engine.feed(Input::Backspace));
                    surviving.pop();
                } else {
                    let key = keys.next_character();
                    perform(&mut text, engine.feed(Input::Char(key)));
                    typed.push(key);
                    surviving.push(key);
                }
            }
            let mut fresh = Engine::new(Config::default());
            assert_eq!(
                text,
                typed_text(&mut fresh, &surviving),
                "surviving keys of {typed:?}"
            );
            assert_eq!(engine.raw_keys, surviving);
        }
    }

    #[test]
    fn reset_clears_the_word() {
        let mut engine = engine();
        engine.feed(Input::Char('a'));
        engine.reset();
        assert!(!engine.feed(Input::Boundary).boundary);
        assert_eq!(typed_text(&mut engine, "a"), "a");
    }

    #[test]
    fn simple_telex_is_telex_without_the_horn_and_breve_shortcuts() {
        // The circumflex, the stroke and the tones come from the same rules…
        for (raw, expected) in [("aa", "â"), ("dd", "đ"), ("vieejt", "việt")] {
            let mut engine = Engine::new(Config {
                method: Method::SimpleTelex,
                ..Config::default()
            });
            assert_eq!(typed_text(&mut engine, raw), expected, "raw {raw:?}");
        }
        // …and the shortcuts this method leaves out stay literal keys.
        for raw in ["aw", "ow", "uw", "uow", "[", "]"] {
            let mut engine = Engine::new(Config {
                method: Method::SimpleTelex,
                ..Config::default()
            });
            assert_eq!(typed_text(&mut engine, raw), raw, "raw {raw:?}");
        }
    }

    #[test]
    fn buffers_stabilize_after_warmup() {
        let mut engine = engine();
        // One word with letter modifiers and a tone (`dduowngf`), one that removes a
        // tone and gets its key back (`asdz`), one the boundary gives back (`asd`):
        // the paths that rewrite, re-insert and restore must not grow the buffers.
        let words = ["dduowngf", "asdz", "asd"];
        for _ in 0..100 {
            for word in words {
                finished_text(&mut engine, word);
            }
        }
        let warmed_up = (
            engine.raw_keys.capacity(),
            engine.shown_text.capacity(),
            engine.scratch.capacity(),
        );

        for _ in 0..1_000 {
            for word in words {
                finished_text(&mut engine, word);
            }
        }
        assert_eq!(
            (
                engine.raw_keys.capacity(),
                engine.shown_text.capacity(),
                engine.scratch.capacity(),
            ),
            warmed_up,
            "reallocation in the hot path"
        );
    }
}
