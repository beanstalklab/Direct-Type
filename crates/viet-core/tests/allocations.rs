//! The allocation promise of spec §2.1 and §9 criterion 4: `feed` does not allocate once
//! the buffers are warm.
//!
//! This file holds exactly one test on purpose. The counter is global, so another test
//! allocating in parallel would land in the measurement; cargo runs each test binary in
//! its own process, and one test means nothing else is running while it counts.
//!
//! `unsafe` is allowed here and nowhere else outside `viet-ffi` (decision D8): a counting
//! allocator is an `unsafe impl GlobalAlloc` and nothing more. It wraps `System` and never
//! touches engine memory.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use viet_core::{Config, Engine, Input, Method};

struct CountingAllocator;

/// Allocations and reallocations since the process started.
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every method delegates to `System` with the same arguments, which is the
// allocator the test binary would use without this wrapper. The only addition is the
// counter, which cannot fail and touches no foreign memory.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: the caller passes a layout valid for the global allocator, and `System`
        // is the allocator being wrapped.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the pointer was allocated by `alloc` above, which delegated to
        // `System`, and the layout is the one it was allocated with.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // A reallocation is a potential allocation in the hot path, which is exactly what
        // the promise forbids, so it is counted.
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: the pointer was allocated by `alloc` above and `new_size` is the
        // requested new size, which `System` validates like any global allocator would.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Words that reach every path of every rule pack: letter modifiers, tone keys, tone
/// removal, doubling undo, the `auto_restore` rewrite, the `w`/`uo` pair, and the keys no
/// rule claims. The last one is long enough to cross the word cap.
const WORDS: [&str; 13] = [
    "dduowngf",
    "vieejt",
    "toasn",
    "asd",
    "aass",
    "thuowr",
    "AAA",
    "a[",
    "sz",
    "uwowr",
    "ddd",
    "toansz",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
];

/// Type one word and end it. Everything here is a `Copy` value or a borrow, so the loop
/// itself allocates nothing.
fn type_word(engine: &mut Engine, word: &str) {
    for character in word.chars() {
        let action = engine.feed(Input::Char(character));
        debug_assert!(action.consumed);
        debug_assert!(action.preedit.is_empty());
    }
    let action = engine.feed(Input::Boundary);
    debug_assert!(!action.consumed);
}

#[test]
fn feed_does_not_allocate_once_the_buffers_are_warm() {
    for method in [Method::Telex, Method::Vni, Method::SimpleTelex] {
        let mut engine = Engine::new(Config {
            method,
            ..Config::default()
        });

        // Warm up until every buffer has reached its high-water mark.
        for _ in 0..100 {
            for word in WORDS {
                type_word(&mut engine, word);
            }
        }

        let before = ALLOCATIONS.load(Ordering::Relaxed);
        for _ in 0..100 {
            for word in WORDS {
                type_word(&mut engine, word);
            }
        }
        let after = ALLOCATIONS.load(Ordering::Relaxed);

        assert_eq!(
            after,
            before,
            "{method:?} allocated {} times after warmup",
            after - before
        );
    }
}
