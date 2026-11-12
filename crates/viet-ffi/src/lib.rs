//! C ABI for the Vietnamese engine. See `include/viet.h`.
//!
//! This is the only crate allowed to contain `unsafe`, and the only place where
//! a panic may cross a foreign boundary — contained by `catch_unwind`.

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
    let _ = catch_unwind(AssertUnwindSafe(|| {
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
    }));
}
