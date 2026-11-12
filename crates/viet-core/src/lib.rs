//! Pure transformation engine.
//!
//! Turns key events into an [`Action`] describing what an input-method frontend
//! should do. This crate must never depend on fcitx5, GTK, Qt, X11, Wayland, the
//! filesystem, or the environment. See `docs/SPEC-engine-core.md`.
//!
//! Status after T1.1: characters are committed immediately and never held in a
//! preedit buffer. Telex/VNI transformation arrives in T1.2 onwards.

#![forbid(unsafe_code)]

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
    /// Keystrokes of the word currently being typed. Kept because backspace and
    /// auto-restore both need to re-derive the word from its raw input (T1.5, T2.1).
    raw_keys: String,
    /// Reused output buffer, so the hot path does not allocate once warmed up.
    commit_buffer: String,
}

impl Engine {
    pub const fn new(config: Config) -> Self {
        Self {
            config,
            raw_keys: String::new(),
            commit_buffer: String::new(),
        }
    }

    /// Feed one key event and return what the frontend should perform.
    pub fn feed(&mut self, input: Input) -> Action<'_> {
        self.commit_buffer.clear();
        match input {
            Input::Char(character) => {
                self.raw_keys.push(character);
                // Every character is consumed so a later language pack can rewrite
                // it, and committed immediately so the application never sees a
                // composition buffer that it would render underlined.
                self.commit_buffer.push(character);
                Action {
                    consumed: true,
                    backspaces: 0,
                    commit: &self.commit_buffer,
                    preedit: "",
                    boundary: false,
                }
            }
            Input::Backspace => {
                self.raw_keys.pop();
                // The application deletes the character itself; the engine only
                // keeps its own copy in sync.
                Action::none()
            }
            Input::Boundary => {
                let ended_word = !self.raw_keys.is_empty();
                self.raw_keys.clear();
                Action {
                    consumed: false,
                    backspaces: 0,
                    commit: "",
                    preedit: "",
                    boundary: ended_word,
                }
            }
            Input::Reset => {
                self.raw_keys.clear();
                Action::none()
            }
        }
    }

    /// Drop all state, for example when focus or language changes.
    pub fn reset(&mut self) {
        self.raw_keys.clear();
        self.commit_buffer.clear();
    }

    pub const fn config(&self) -> &Config {
        &self.config
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        Engine::new(Config::default())
    }

    #[test]
    fn character_is_committed_immediately() {
        let mut engine = engine();
        let action = engine.feed(Input::Char('a'));
        assert!(action.consumed);
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
    fn boundary_reports_the_end_of_a_word() {
        let mut engine = engine();
        engine.feed(Input::Char('a'));
        let action = engine.feed(Input::Boundary);
        assert!(action.boundary);
        assert!(!action.consumed);
        assert_eq!(action.commit, "");
    }

    #[test]
    fn boundary_on_empty_buffer_does_not_report_a_word() {
        let mut engine = engine();
        let action = engine.feed(Input::Boundary);
        assert!(!action.boundary);
    }

    #[test]
    fn reset_clears_the_word() {
        let mut engine = engine();
        engine.feed(Input::Char('a'));
        engine.reset();
        assert!(!engine.feed(Input::Boundary).boundary);
    }

    #[test]
    fn backspace_keeps_the_raw_word_in_sync() {
        let mut engine = engine();
        engine.feed(Input::Char('a'));
        engine.feed(Input::Backspace);
        // The raw word is empty again, so this boundary has nothing to finish.
        assert!(!engine.feed(Input::Boundary).boundary);
    }

    #[test]
    fn commit_buffer_capacity_stabilizes() {
        let mut engine = engine();
        for _ in 0..100 {
            engine.feed(Input::Char('a'));
        }
        let warmed_up_capacity = engine.commit_buffer.capacity();
        for _ in 0..1_000 {
            engine.feed(Input::Char('a'));
        }
        assert_eq!(
            engine.commit_buffer.capacity(),
            warmed_up_capacity,
            "reallocation in the hot path"
        );
    }
}
