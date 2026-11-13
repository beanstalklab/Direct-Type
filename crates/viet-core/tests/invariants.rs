//! Properties that hold for every configuration and every input.
//!
//! This file uses only the public API, which is what a frontend has. The properties that
//! need the engine's own state — a restore returns exactly the keystrokes, a delete
//! re-derives the shorter word — live next to that state, in `src/lib.rs` and in the
//! corpus test of `viet-cli`.
//!
//! Nothing here asserts a Vietnamese spelling. These are the promises in spec §2.1: no
//! panic, `reset` is total, a key that is consumed commits text, `preedit` stays empty,
//! and a delete never reaches past the word on screen.

use viet_core::{Action, Config, Engine, Input, Method, ToneStyle};

/// Every configuration the engine offers: three methods, two tone styles, and the three
/// switches at both settings.
fn every_configuration() -> Vec<Config> {
    let mut configurations = Vec::new();
    for method in [Method::Telex, Method::SimpleTelex, Method::Vni] {
        for tone_style in [ToneStyle::Old, ToneStyle::New] {
            for auto_restore in [false, true] {
                for spell_check in [false, true] {
                    for free_tone_mark in [false, true] {
                        configurations.push(Config {
                            method,
                            tone_style,
                            auto_restore,
                            spell_check,
                            free_tone_mark,
                        });
                    }
                }
            }
        }
    }
    configurations
}

/// Everything a keyboard or a paste can send: the keys each rule pack claims, the
/// Vietnamese letters a layout produces directly, controls, a combining mark, and
/// characters far outside any alphabet.
fn hostile_characters() -> Vec<char> {
    let mut characters: Vec<char> = Vec::new();
    characters.extend("aeiouydwsfrxjztpqkngmch[]0123456789".chars());
    characters.extend("ĐđƯưƠơĂăÂâÊêÔôẾếẬậẶặỸỹ".chars());
    characters.extend([
        '\u{0300}', '\u{0323}', '\u{0302}', '\n', '\t', '\u{0}', '\u{7f}',
    ]);
    characters.extend(['😀', '\u{FFFD}', '\u{10FFFF}']);
    characters
}

/// A deterministic keystroke stream, so a failing input can be typed again without a
/// random number dependency.
struct Keys {
    state: u64,
    characters: Vec<char>,
}

impl Keys {
    fn new(seed: u64) -> Self {
        Self {
            state: seed,
            characters: hostile_characters(),
        }
    }

    fn next_number(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state >> 33
    }

    /// A keystroke, weighted so most of them are characters and some are edges.
    fn next_input(&mut self) -> Input {
        match self.next_number() % 8 {
            0 => Input::Backspace,
            1 => Input::Boundary,
            2 => Input::Reset,
            _ => {
                let index = self.next_number() as usize % self.characters.len();
                Input::Char(self.characters[index])
            }
        }
    }
}

/// A combining mark is what a decomposed (NFD) Vietnamese letter is made of.
fn is_combining_mark(character: char) -> bool {
    ('\u{0300}'..='\u{036F}').contains(&character)
}

#[test]
fn no_configuration_panics_on_any_input() {
    for configuration in every_configuration() {
        let mut engine = Engine::new(configuration);
        let mut keys = Keys::new(0x5eed);
        for _ in 0..400 {
            let action = engine.feed(keys.next_input());
            // The two promises every action keeps, whatever the input was.
            assert!(
                action.preedit.is_empty(),
                "the engine never holds a preedit"
            );
        }
        // A long run without a boundary must not break the word cap, and the calls after
        // it must still work.
        for _ in 0..500 {
            engine.feed(Input::Char('a'));
        }
        engine.feed(Input::Boundary);
        engine.feed(Input::Reset);
        assert_eq!(engine.feed(Input::Reset), Action::none());
    }
}

#[test]
fn a_reset_leaves_no_state_behind() {
    // The same keys typed after a reset must produce the same actions as on a fresh
    // engine, or the word before the reset is still influencing what the user sees.
    let keys = "vieejt asd aaa thuow d9uo7ng";
    for configuration in every_configuration() {
        let mut used = Engine::new(configuration);
        for character in "dduowng\u{0301} asdf".chars() {
            used.feed(input_for(character));
        }
        used.reset();

        let mut fresh = Engine::new(configuration);
        for character in keys.chars() {
            let from_used = used.feed(input_for(character));
            let from_fresh = fresh.feed(input_for(character));
            assert_eq!(
                from_used, from_fresh,
                "after a reset, {character:?} behaved differently"
            );
        }
    }
}

fn input_for(character: char) -> Input {
    if character.is_whitespace() {
        Input::Boundary
    } else {
        Input::Char(character)
    }
}

#[test]
fn a_character_action_is_consumed_and_precomposed() {
    for configuration in every_configuration() {
        let mut engine = Engine::new(configuration);
        for character in hostile_characters() {
            if is_combining_mark(character) {
                // A combining mark typed on purpose is the caller's text: the engine
                // echoes it rather than composing with it (spec §2.1, decision D1).
                engine.feed(Input::Char(character));
                continue;
            }
            let action = engine.feed(Input::Char(character));
            assert!(
                action.consumed,
                "a character must never be forwarded twice: {character:?}"
            );
            for committed in action.commit.chars() {
                assert!(
                    !is_combining_mark(committed),
                    "the output of {character:?} is not precomposed"
                );
            }
        }
    }
}

#[test]
fn a_boundary_is_always_forwarded() {
    // The boundary key belongs to the application: the engine may rewrite the word
    // before it, but the space, tab or enter itself has to reach the application.
    for configuration in every_configuration() {
        let mut engine = Engine::new(configuration);
        for character in "dduowngf".chars() {
            engine.feed(Input::Char(character));
        }
        let action = engine.feed(Input::Boundary);
        assert!(!action.consumed);
        assert!(action.boundary);
        assert_eq!(engine.feed(Input::Boundary), Action::none());
    }
}

#[test]
fn the_engine_never_deletes_more_than_it_shows() {
    // A frontend that honours `backspaces` by popping its own buffer would corrupt it
    // if the engine asked for more deletions than the word has characters.
    for configuration in every_configuration() {
        let mut engine = Engine::new(configuration);
        let mut text = String::new();
        let mut keys = Keys::new(0xbadc0de);
        for _ in 0..400 {
            let action = engine.feed(keys.next_input());
            assert!(
                action.backspaces as usize <= text.chars().count(),
                "the engine asked to delete {} characters of {text:?}",
                action.backspaces
            );
            for _ in 0..action.backspaces {
                text.pop();
            }
            text.push_str(action.commit);
        }
    }
}
