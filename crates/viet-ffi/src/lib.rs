//! C ABI for the Vietnamese engine. See `include/viet.h`.
//!
//! This crate and the allocation-counting test (spec decision D8) are the only places
//! allowed to contain `unsafe`, and this is the only place where a panic may reach a
//! foreign boundary — contained by `catch_unwind`, because unwinding into C is undefined
//! behaviour.
//!
//! Strings are length-delimited in bytes and are **not** NUL-terminated: the engine hands
//! out a pointer into its own buffer, so a C caller must read `commit_length` bytes and
//! never call `strlen`.

#![allow(non_camel_case_types)]

use std::ffi::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};

use viet_core::{Config, Engine, Input, Method, ToneStyle};

/// Opaque to C; C sees only `viet_engine_t *`.
pub struct viet_engine {
    transform_engine: Engine,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct viet_config_t {
    pub method: i32,
    pub tone_style: i32,
    pub auto_restore: i32,
    pub spell_check: i32,
    pub free_tone_mark: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct viet_input_t {
    pub kind: i32,
    pub code_point: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct viet_action_t {
    pub consumed: i32,
    pub backspaces: u16,
    pub commit_length: u32,
    pub preedit_length: u32,
    pub boundary: i32,
    pub reserved_flags: u32,
}

impl viet_action_t {
    const fn none() -> Self {
        Self {
            consumed: 0,
            backspaces: 0,
            commit_length: 0,
            preedit_length: 0,
            boundary: 0,
            reserved_flags: 0,
        }
    }
}

fn decode_configuration(configuration: &viet_config_t) -> Config {
    Config {
        method: match configuration.method {
            1 => Method::SimpleTelex,
            2 => Method::Vni,
            _ => Method::Telex,
        },
        tone_style: if configuration.tone_style == 0 {
            ToneStyle::Old
        } else {
            ToneStyle::New
        },
        auto_restore: configuration.auto_restore != 0,
        spell_check: configuration.spell_check != 0,
        free_tone_mark: configuration.free_tone_mark != 0,
    }
}

/// Run `body` with panics contained. `None` means it panicked and the caller's no-op
/// action stands.
fn catching_panic<R>(body: impl FnOnce() -> R) -> Option<R> {
    catch_unwind(AssertUnwindSafe(body)).ok()
}

// Asks the next `feed` to panic, so the containment test drives the real function instead
// of a closure that stands in for it. Never compiled into a shipped build.
#[cfg(test)]
thread_local! {
    static PANIC_NEXT_FEED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn maybe_panic_for_test() {
    PANIC_NEXT_FEED.with(|flag| {
        if flag.replace(false) {
            panic!("forced panic: the C boundary has to contain this");
        }
    });
}

fn decode_input(input: &viet_input_t) -> Option<Input> {
    match input.kind {
        0 => char::from_u32(input.code_point).map(Input::Char),
        1 => Some(Input::Backspace),
        2 => Some(Input::Boundary),
        3 => Some(Input::Reset),
        _ => None,
    }
}

/// Create an engine. Passing NULL uses the default configuration.
///
/// # Safety
/// `configuration` must be NULL or point to a valid `viet_config_t`.
#[no_mangle]
pub unsafe extern "C" fn viet_engine_new(configuration: *const viet_config_t) -> *mut viet_engine {
    let config = if configuration.is_null() {
        Config::default()
    } else {
        decode_configuration(&*configuration)
    };
    Box::into_raw(Box::new(viet_engine {
        transform_engine: Engine::new(config),
    }))
}

/// Destroy an engine. Passing NULL is a no-op.
///
/// # Safety
/// `engine` must be NULL or a pointer returned by `viet_engine_new` that has not
/// already been freed.
#[no_mangle]
pub unsafe extern "C" fn viet_engine_free(engine: *mut viet_engine) {
    if engine.is_null() {
        return;
    }
    drop(Box::from_raw(engine));
}

/// Drop all composition state.
///
/// # Safety
/// `engine` must be NULL or a valid engine pointer.
#[no_mangle]
pub unsafe extern "C" fn viet_engine_reset(engine: *mut viet_engine) {
    if engine.is_null() {
        return;
    }
    (*engine).transform_engine.reset();
}

/// Apply a configuration at runtime.
///
/// The word in progress is dropped: a settings change is not a typing event, and the text on
/// screen was derived with the previous rule pack. Passing NULL applies the default
/// configuration, the same as `viet_engine_new(NULL)`.
///
/// # Safety
/// `engine` must be NULL or a valid engine pointer; `configuration` must be NULL or point to
/// a valid `viet_config_t`.
#[no_mangle]
pub unsafe extern "C" fn viet_engine_set_config(
    engine: *mut viet_engine,
    configuration: *const viet_config_t,
) {
    if engine.is_null() {
        return;
    }
    let config = if configuration.is_null() {
        Config::default()
    } else {
        // SAFETY: the caller guarantees provenance, as documented above.
        decode_configuration(&*configuration)
    };
    let transform_engine = &mut (*engine).transform_engine;
    let _ = catching_panic(|| {
        transform_engine.set_config(config);
        transform_engine.reset();
    });
}

/// Feed one key event.
///
/// On any invalid argument or internal panic, `out_action` is set to a no-op
/// action and the string pointers are set to NULL.
///
/// # Safety
/// `engine` must be a valid engine pointer; `input` must point to a valid
/// `viet_input_t`; `out_action` must be non-NULL; `out_commit` and `out_preedit`
/// may be NULL. The returned string pointers are valid only until the next
/// `viet_engine_feed` call on the same engine.
#[no_mangle]
pub unsafe extern "C" fn viet_engine_feed(
    engine: *mut viet_engine,
    input: *const viet_input_t,
    out_action: *mut viet_action_t,
    out_commit: *mut *const c_char,
    out_preedit: *mut *const c_char,
) {
    if out_action.is_null() {
        return;
    }
    *out_action = viet_action_t::none();
    if !out_commit.is_null() {
        *out_commit = std::ptr::null();
    }
    if !out_preedit.is_null() {
        *out_preedit = std::ptr::null();
    }
    if engine.is_null() || input.is_null() {
        return;
    }

    let transform_engine = &mut (*engine).transform_engine;
    let Some(input_event) = decode_input(&*input) else {
        return;
    };

    // All writes happen inside the guard: `Action` borrows the engine, so it
    // cannot escape the closure. A panic leaves the no-op action set above.
    let _ = catching_panic(|| {
        #[cfg(test)]
        maybe_panic_for_test();
        let action = transform_engine.feed(input_event);
        *out_action = viet_action_t {
            consumed: i32::from(action.consumed),
            backspaces: action.backspaces,
            commit_length: action.commit.len() as u32,
            preedit_length: action.preedit.len() as u32,
            boundary: i32::from(action.boundary),
            reserved_flags: 0,
        };
        if !out_commit.is_null() {
            *out_commit = action.commit.as_ptr() as *const c_char;
        }
        if !out_preedit.is_null() {
            *out_preedit = action.preedit.as_ptr() as *const c_char;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character_input(character: char) -> viet_input_t {
        viet_input_t {
            kind: 0,
            code_point: u32::from(character),
        }
    }

    fn boundary_input() -> viet_input_t {
        viet_input_t {
            kind: 2,
            code_point: 0,
        }
    }

    /// One action of the ABI, copied into Rust-owned memory: what a frontend sees.
    struct Step {
        consumed: i32,
        backspaces: u16,
        commit: String,
        boundary: i32,
    }

    fn feed(engine: *mut viet_engine, input: viet_input_t) -> Step {
        let mut action = viet_action_t::none();
        let mut commit: *const c_char = std::ptr::null();
        // SAFETY: the pointers are valid for the call, and `engine` came from
        // `viet_engine_new`.
        unsafe {
            viet_engine_feed(
                engine,
                &input,
                &mut action,
                &mut commit,
                std::ptr::null_mut(),
            );
        }
        let commit = if commit.is_null() || action.commit_length == 0 {
            String::new()
        } else {
            // SAFETY: the span is engine-owned, valid until the next feed on the same
            // engine, and `commit_length` bytes long.
            let bytes = unsafe {
                std::slice::from_raw_parts(commit as *const u8, action.commit_length as usize)
            };
            std::str::from_utf8(bytes)
                .expect("the commit is UTF-8")
                .to_string()
        };
        Step {
            consumed: action.consumed,
            backspaces: action.backspaces,
            commit,
            boundary: action.boundary,
        }
    }

    /// Apply one step the way the addon will.
    fn perform(text: &mut String, step: Step) {
        for _ in 0..step.backspaces {
            text.pop();
        }
        text.push_str(&step.commit);
    }

    /// Type a word and end it, returning the text the application shows.
    fn type_keys(engine: *mut viet_engine, keys: &str) -> String {
        let mut text = String::new();
        for key in keys.chars() {
            let step = feed(engine, character_input(key));
            assert_eq!(step.consumed, 1, "a typed key is consumed: {key:?}");
            perform(&mut text, step);
        }
        let step = feed(engine, boundary_input());
        assert_eq!(step.consumed, 0, "the boundary key is forwarded");
        assert_eq!(step.boundary, 1);
        perform(&mut text, step);
        text
    }

    #[test]
    fn the_abi_types_a_word_the_way_the_engine_does() {
        // SAFETY: a NULL configuration selects the default, and the pointer is freed once.
        let engine = unsafe { viet_engine_new(std::ptr::null()) };
        assert!(!engine.is_null());
        assert_eq!(type_keys(engine, "vieejt"), "việt");
        assert_eq!(type_keys(engine, "dduowngf"), "đường");
        assert_eq!(
            type_keys(engine, "asd"),
            "asd",
            "auto_restore is on by default"
        );
        // SAFETY: the pointer came from `viet_engine_new`.
        unsafe { viet_engine_free(engine) };
    }

    #[test]
    fn the_commit_span_is_counted_in_bytes() {
        // `â` is two bytes, so a caller that trusted a character count or `strlen` would
        // read the wrong span.
        // SAFETY: as above.
        let engine = unsafe { viet_engine_new(std::ptr::null()) };
        let mut action = viet_action_t::none();
        let mut commit: *const c_char = std::ptr::null();
        for key in ['a', 'a'] {
            let input = character_input(key);
            // SAFETY: as above.
            unsafe {
                viet_engine_feed(
                    engine,
                    &input,
                    &mut action,
                    &mut commit,
                    std::ptr::null_mut(),
                );
            }
        }
        assert_eq!(action.backspaces, 1);
        assert_eq!(action.commit_length, "â".len() as u32);
        assert_eq!(action.preedit_length, 0, "there is no composition buffer");
        // SAFETY: the span is engine-owned and `commit_length` bytes long.
        let bytes = unsafe {
            std::slice::from_raw_parts(commit as *const u8, action.commit_length as usize)
        };
        assert_eq!(std::str::from_utf8(bytes), Ok("â"));
        unsafe { viet_engine_free(engine) };
    }

    #[test]
    fn unknown_inputs_and_null_arguments_are_a_no_op() {
        // SAFETY: as above.
        let engine = unsafe { viet_engine_new(std::ptr::null()) };
        let mut action = viet_action_t::none();
        let mut commit: *const c_char = std::ptr::null();

        // An unknown kind decodes to nothing and leaves the no-op action in place.
        let unknown = viet_input_t {
            kind: 99,
            code_point: u32::from('a'),
        };
        // SAFETY: as above.
        unsafe {
            viet_engine_feed(
                engine,
                &unknown,
                &mut action,
                &mut commit,
                std::ptr::null_mut(),
            );
        }
        assert_eq!(action.consumed, 0);
        assert_eq!(action.backspaces, 0);
        assert!(commit.is_null());

        // A NULL engine is a no-op rather than a crash, which is what a frontend that has
        // already torn its engine down will pass.
        let character = character_input('a');
        // SAFETY: a NULL engine is documented as a no-op.
        unsafe {
            viet_engine_feed(
                std::ptr::null_mut(),
                &character,
                &mut action,
                &mut commit,
                std::ptr::null_mut(),
            );
        }
        assert_eq!(action.consumed, 0);

        // NULL outputs are allowed, and a NULL action is refused before anything else.
        // SAFETY: NULL outputs are documented as optional.
        unsafe {
            viet_engine_feed(
                engine,
                &character,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            viet_engine_reset(std::ptr::null_mut());
            viet_engine_free(std::ptr::null_mut());
            viet_engine_free(engine);
        }
    }

    #[test]
    fn a_configuration_can_be_applied_at_runtime() {
        let telex = viet_config_t {
            method: 0,
            tone_style: 1,
            auto_restore: 1,
            spell_check: 0,
            free_tone_mark: 1,
        };
        let vni = viet_config_t { method: 2, ..telex };

        // SAFETY: the configuration is a valid struct and the pointer is freed once.
        let engine = unsafe { viet_engine_new(&telex) };
        assert_eq!(type_keys(engine, "vieejt"), "việt");

        // The same engine, now speaking VNI. `vieejt` would be nonsense there, so the test
        // types a VNI word instead.
        // SAFETY: as above.
        unsafe { viet_engine_set_config(engine, &vni) };
        assert_eq!(type_keys(engine, "d9u7o7ng2"), "đường");

        // NULL means the defaults, which are Telex with the new tone style.
        // SAFETY: a NULL configuration is documented as the default.
        unsafe { viet_engine_set_config(engine, std::ptr::null()) };
        assert_eq!(type_keys(engine, "vieejt"), "việt");

        // A NULL engine is a no-op rather than a crash.
        // SAFETY: a NULL engine is documented as a no-op.
        unsafe { viet_engine_set_config(std::ptr::null_mut(), &telex) };
        unsafe { viet_engine_free(engine) };
    }

    #[test]
    fn applying_a_configuration_drops_the_word_in_progress() {
        // A change of rule pack must not leave a half-typed word to be finished by the new
        // rules: the session is reset instead.
        // SAFETY: as above.
        let engine = unsafe { viet_engine_new(std::ptr::null()) };
        type_keys(engine, "viee"); // a word in progress, no boundary yet
        let configuration = viet_config_t {
            method: 2,
            tone_style: 1,
            auto_restore: 1,
            spell_check: 0,
            free_tone_mark: 1,
        };
        // SAFETY: as above.
        unsafe { viet_engine_set_config(engine, &configuration) };
        // The next key starts a new word, so `d9` is `đ` rather than VNI marks on `viê`.
        assert_eq!(type_keys(engine, "d9"), "đ");
        unsafe { viet_engine_free(engine) };
    }

    #[test]
    fn a_panic_in_the_engine_is_contained() {
        // SAFETY: as above.
        let engine = unsafe { viet_engine_new(std::ptr::null()) };
        let input = character_input('a');
        let mut action = viet_action_t::none();
        let mut commit: *const c_char = std::ptr::null();

        PANIC_NEXT_FEED.with(|flag| flag.set(true));
        // SAFETY: as above. The forced panic must not unwind through this call.
        unsafe {
            viet_engine_feed(
                engine,
                &input,
                &mut action,
                &mut commit,
                std::ptr::null_mut(),
            );
        }
        assert_eq!(action.consumed, 0, "a panicking feed reports a no-op");
        assert_eq!(action.backspaces, 0);
        assert_eq!(action.commit_length, 0);
        assert!(commit.is_null());

        // The engine is still usable afterwards: the panic is reported, not fatal.
        assert_eq!(type_keys(engine, "aa"), "â");
        unsafe { viet_engine_free(engine) };
    }
}
