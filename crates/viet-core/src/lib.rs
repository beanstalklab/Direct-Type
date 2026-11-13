//! Pure transformation engine.
//!
//! Turns key events into an [`Action`] describing what an input-method frontend
//! should do. This crate must never depend on fcitx5, GTK, Qt, X11, Wayland, the
//! filesystem, or the environment. See `docs/SPEC-engine-core.md`.
//!
//! Status: Telex letter modifiers work (T1.2). Characters are committed
//! immediately and never held in a preedit buffer, so a modifier rewrites what the
//! application already shows instead of building a composition string. Tone keys
//! (T1.4), VNI (T1.7) and Simple Telex's restricted modifier set (T1.8) are not
//! implemented yet.

#![forbid(unsafe_code)]

mod syllable;
mod telex;
mod tone;
mod unicode;

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
    /// Restore the raw keystrokes when the buffered word is not valid Vietnamese.
    pub auto_restore: bool,
    /// Do not transform words that cannot become valid Vietnamese.
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

/// One key event handed to the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Printable character after the frontend applied shift and layout mapping.
    Char(char),
    Backspace,
    /// Space, tab, enter, punctuation, or focus loss: ends the current word.
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
                let ended_word = !self.raw_keys.is_empty();
                self.clear_word();
                Action {
                    consumed: false,
                    backspaces: 0,
                    commit: "",
                    preedit: "",
                    boundary: ended_word,
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

        let (shared_characters, shared_bytes) = shared_prefix(&self.shown_text, &self.scratch);
        // Characters, not bytes: the ABI counts deletions in characters.
        let backspaces = u16::try_from(self.shown_text.chars().count() - shared_characters)
            // Unreachable for a real word; clamping keeps the 16-bit ABI field honest
            // instead of wrapping to a small number that would corrupt the word.
            .unwrap_or(u16::MAX);

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

    /// Put the transformation of the current word into `scratch`.
    fn transform_into_scratch(&mut self) {
        match self.config.method {
            Method::Telex => telex::apply_letter_modifiers(&self.raw_keys, &mut self.scratch),
            // Simple Telex has a narrower modifier set (T1.8) and VNI uses digit
            // keys (T1.7). Until those land, their words reach the application
            // unchanged rather than half-transformed by the wrong rule pack.
            Method::SimpleTelex | Method::Vni => {
                self.scratch.clear();
                self.scratch.push_str(&self.raw_keys);
            }
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
            let action = engine.feed(Input::Char(character));
            for _ in 0..action.backspaces {
                text.pop();
            }
            text.push_str(action.commit);
        }
        text
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
    fn backspace_on_an_empty_word_is_forwarded() {
        let mut engine = engine();
        assert_eq!(engine.feed(Input::Backspace), Action::none());
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
    fn unimplemented_methods_pass_letters_through() {
        // T1.8 will give Simple Telex its narrower rule set; until then the engine
        // must not apply Telex rules under another method's name.
        let mut engine = Engine::new(Config {
            method: Method::SimpleTelex,
            ..Config::default()
        });
        assert_eq!(typed_text(&mut engine, "aa"), "aa");
    }

    #[test]
    fn buffers_stabilize_after_warmup() {
        let mut engine = engine();
        for _ in 0..100 {
            typed_text(&mut engine, "thuowng");
            engine.reset();
        }
        let warmed_up = (
            engine.raw_keys.capacity(),
            engine.shown_text.capacity(),
            engine.scratch.capacity(),
        );

        for _ in 0..1_000 {
            typed_text(&mut engine, "thuowng");
            engine.reset();
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
