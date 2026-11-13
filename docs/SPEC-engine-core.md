# Spec: `engine-core` — Vietnamese transformation engine

Status: **draft, awaiting approval** · Module id: `engine-core` · Depends on: none

## 1. Objective

A pure, deterministic library that turns a stream of key events into an `Action` describing what an IME frontend should do. It knows Vietnamese orthography and nothing else. It never talks to fcitx5, X11, Wayland, or the filesystem.

Success for this module: given a corpus of input keystrokes, it produces the same final text a correct Vietnamese IME produces, in under 1 ms per keystroke, with no panics on arbitrary input.

Non-goals: key capture, preedit rendering, app compatibility, settings UI, macros, encoding conversion, languages other than Vietnamese (the API must not, however, assume Vietnamese in the frontend layer).

### Users

1. `ime-addon` (v1 frontend) — the only consumer in v1.
2. `viet-cli` — corpus runner and manual testing.
3. Future frontends (IBus engine, privileged helper) — the C ABI must stay general enough to serve them.

## 2. Public interface

### 2.1 Rust API

```rust
pub struct Config {
    pub method: Method,          // Telex | SimpleTelex | Vni
    pub tone_style: ToneStyle,   // Old | New  (hoà/hoà vs hòa/hòa)
    pub auto_restore: bool,      // invalid word -> restore raw keystrokes
    pub spell_check: bool,       // do not transform non-Vietnamese-looking words
    pub free_tone_mark: bool,    // accept tone key anywhere in the syllable
}

pub enum Input {
    Char(char),                  // printable character, lowercase or uppercase
    Backspace,
    Boundary,                    // space, tab, enter, punctuation, focus loss
    Reset,
}

pub struct Action<'a> {
    pub consumed: bool,          // frontend must not forward the original key
    pub backspaces: u16,         // chars to delete before committing
    pub commit: &'a str,         // text the app should end up with
    pub preedit: &'a str,        // what to show while composing (may be empty)
    pub boundary: bool,          // a word just ended
}

pub struct Engine { /* private */ }

impl Engine {
    pub fn new(config: Config) -> Self;
    pub fn feed(&mut self, input: Input) -> Action<'_>;
    pub fn reset(&mut self);
    pub fn config(&self) -> &Config;
    pub fn set_config(&mut self, config: Config);
}
```

Invariants:

- `feed` is allocation-free in steady state: all buffers are pre-allocated and reused; `Action` borrows from the engine.
- Deterministic: same config + same input sequence → byte-identical `Action` sequence.
- Never panics, for any `Input` sequence (enforced by fuzzing).
- Output is always NFC (precomposed Vietnamese), never NFD, never mixed.

### 2.2 C ABI (consumed by the C++ addon)

`crates/viet-ffi/include/viet.h`:

```c
typedef struct viet_engine viet_engine_t;

/* method: 0 = telex, 1 = simple_telex, 2 = vni
   tone_style: 0 = old, 1 = new */
typedef struct {
  int32_t method;
  int32_t tone_style;
  int32_t auto_restore;
  int32_t spell_check;
  int32_t free_tone_mark;
} viet_config_t;

/* kind: 0 = char, 1 = backspace, 2 = boundary, 3 = reset */
typedef struct {
  int32_t  kind;
  uint32_t code_point;   /* Unicode scalar value when kind == 0 */
} viet_input_t;

typedef struct {
  int32_t  consumed;
  uint16_t backspaces;
  uint32_t commit_length;
  uint32_t preedit_length;
  int32_t  boundary;
  uint32_t reserved_flags; /* must be 0 in v1 */
} viet_action_t;

viet_engine_t *viet_engine_new(const viet_config_t *configuration);
void           viet_engine_free(viet_engine_t *engine);
void           viet_engine_reset(viet_engine_t *engine);

/* out_action and both string spans point into engine-owned memory,
   valid until the next viet_engine_feed call. No allocation crosses the boundary. */
void viet_engine_feed(viet_engine_t *engine, const viet_input_t *input,
                      viet_action_t *out_action,
                      const char **out_commit, const char **out_preedit);
```

Rules: `extern "C"`, `#[no_mangle]`, no unwind across the boundary (`catch_unwind` at the edge, return a safe no-op action), no ownership transfer, no callbacks.

## 3. Engine behaviour (the part that must be right)

### 3.1 Input classification

Keys arrive as characters, not keysyms. `ime-addon` maps keysyms → `char` (including shifted variants) before calling the core. The core never needs modifier state.

### 3.2 Methods

| Method | Tone keys | Letter modifiers |
|---|---|---|
| Telex | `s` sắc, `f` huyền, `r` hỏi, `x` ngã, `j` nặng; `z` removes tone | `aa→â`, `ee→ê`, `oo→ô`, `w→ư`, `aw→ă`, `ow→ơ`, `uw→ư`, `dd→đ`, `[→ơ`, `]→ư` |
| SimpleTelex | same | `aa→â`, `ee→ê`, `oo→ô`, `w→ư`, `dd→đ` only |
| VNI | `1..5` tones, `0` removes tone | `6` circumflex (`a6→â`), `7` horn (`o7→ơ`, `u7→ư`), `8` breve (`a8→ă`), `9` (`d9→đ`) |

Doubling a tone key removes the tone and keeps the letter (`ass → as`, `a11 → a1` in VNI semantics must be defined by corpus, not guessed).

Letter modifiers in Telex, in the detail T1.2 pinned down:

- Doubling undoes the modifier and keeps the key literal: `aaa → aa`, `ddd → dd`, `ww → w`, `uoww → uow`.
- `w` after `o` horns both letters of the `uo` pair, because one keystroke has to reach `ươ`: `uow → ươ`, `uwow → ươ`.
- `[` and `]` insert `ơ` and `ư` themselves; they are listed as letters above, not as a modification of a preceding letter (`a[` is `aơ`, not `ă`).
- A letter that already carries a different mark keeps it and the key stays literal: `awa → ăa`, `oow → ôw`.
- Case follows the letter that carries the mark, which is the first of the pair: `AA → Â`, `aA → â`, `VIEE → VIÊ`.

### 3.3 Syllable model

Parse the buffer into `onset · nucleus · coda`. `qu` and `gi` are onsets, not vowel
clusters. This is what makes `quà`, `quý`, `già`, `gì` place tones correctly. `gi`
is an onset only when a vowel follows it (`già`), so `gì` keeps `i` as its nucleus.

There is deliberately no separate medial field (decided in T1.3). A glide stays in
the nucleus: `hoa` has two nucleus vowels, which is exactly why `hòa` and `hoà`
disagree, and `khuỷu` has three, which is why its tone lands on the middle one.
Splitting the glide out would give `hoa` one nucleus vowel and `khuỷu` two, and both
tone placements would be wrong. `qu` and `gi` are onsets, which is the only place
the glide does not belong to the nucleus.

The parser checks structure, not spelling: at most three onset consonants, one to
three nucleus vowels, at most two coda consonants, and no vowel after the coda.
Whether `xt` is a real coda, or `ngh` a real onset, is a spelling question that
`spell_check` answers in T1.5.

### 3.4 Tone placement

Order of rules:

1. If any nucleus vowel carries a quality diacritic (`ă â ê ô ơ ư`), place the tone on it; if several, use the last one that the orthography allows (`ướ`, `uyễ`).
2. Otherwise, if the syllable has a coda, place the tone on the last nucleus vowel (`toán`, `tiếng`).
3. Otherwise (open syllable): one vowel → that vowel; two → first (`cái`, `của`); three → second (`khuỷu`).
4. `ToneStyle::Old` vs `New` changes only the `oa / oe / uy` open-syllable case (`hoà` vs `hòa`, `thuỷ` vs `thủy`). Every other open pair keeps the first vowel under both styles: `cái` and `của` must not move.

Every rule above must be backed by at least three corpus cases, including a counter-example. If implementation and corpus disagree, the corpus wins only when it cites a source in its comment.

### 3.5 Commit and restore

- **Passthrough commit:** a character that needs no transformation is committed
  immediately (`consumed = true`, `commit` = that character) and is never held in
  preedit. This is what makes the no-underline promise structural: the application
  only ever receives committed text, never a composition buffer.
- Word boundary: commit the transformed buffer, clear state, `boundary = true`.
- Invalid syllable + `auto_restore` → commit the raw keystrokes exactly as typed.
- Invalid syllable + `!auto_restore` → commit the best transformed attempt.
- `spell_check` on: a buffer that cannot become a valid Vietnamese syllable is passed through untouched.
- Backspace during composition removes the last *raw* key, then re-derives the transformation from scratch. This is why the engine keeps the raw key sequence alongside the transformed text.

## 4. Commands

Run from the repository root. `cargo` is user-local on this host, so source
`~/.cargo/env` first; other hosts only need a stable toolchain with clippy.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p viet-core --test corpus -- --nocapture
cargo run -p viet-cli -- type --method telex --tone-style new "vieejt"
cargo run -p viet-cli -- check tests/corpus
cargo bench -p viet-core            # after the first correctness milestone
```

`viet-cli` exit codes: `0` all corpus cases pass, `1` mismatch, `2` malformed corpus. This makes the corpus usable as a CI gate from day one.

## 5. Project structure

```
Cargo.toml                      workspace: members = crates/*
rust-toolchain.toml             pinned stable + clippy + rustfmt
crates/viet-core/
  src/lib.rs                    public API, Engine, Action
  src/config.rs                 Config, Method, ToneStyle
  src/input.rs                  Input classification, tone/modifier keys
  src/buffer.rs                 raw key sequence + transformed view
  src/syllable.rs               onset/medial/nucleus/coda parsing + validity
  src/tone.rs                   tone placement rules 3.4
  src/telex.rs
  src/vni.rs
  src/unicode.rs               Vietnamese precomposed table, case handling
  tests/corpus.rs               runs tests/corpus/*.tsv through the engine
  tests/invariants.rs           property tests
  fuzz/                         cargo-fuzz target (dev only)
crates/viet-cli/src/main.rs
crates/viet-ffi/
  src/lib.rs
  include/viet.h
tests/corpus/telex-new.tsv       filename = method + tone style
tests/corpus/telex-old.tsv
tests/corpus/simple-telex-new.tsv
tests/corpus/vni-new.tsv
scripts/corpus_tool.py          Python 3.14 stdlib only: sort, dedupe, validate TSV
```

`tests/corpus` lives at the workspace root, not inside `viet-core`, because the addon's integration tests will reuse it.

### Corpus format

Tab-separated, `#` comments, one case per line, no escapes needed:

```
# input	expected	# comment (optional third column)
vieejt	việt
hoaf	hoà	old tone style file
quyeenf	quyền
```

File names select the configuration: `method[-topic][-style].tsv`, where method is
`telex`, `simple-telex` or `vni`, style is `old` or `new`, and the topic is free
text the runner ignores. A missing style means the default (`new`). So
`telex-letters.tsv` runs full Telex with the new style, `telex-new.tsv` runs the
same configuration for tone cases, and `simple-telex-old.tsv` runs Simple Telex
with old placement.

The runner feeds each input character as `Input::Char`, then `Input::Boundary`, and compares the concatenation of `commit` values *after* honouring `backspaces`, so a case can express a modifier that rewrites an earlier character. Uppercase and mixed case are explicit cases, not implicit behaviour.

## 6. Code style

```rust
// Tone placement: quality diacritic wins, then coda, then open-syllable rules.
pub(crate) fn tone_index(syllable: &Syllable, style: ToneStyle) -> Option<usize> {
    if let Some(index) = last_quality_vowel(syllable) {
        return Some(index);
    }
    if syllable.has_coda() {
        return syllable.nucleus_length.checked_sub(1);
    }
    match syllable.nucleus_length {
        0 => None,
        1 => Some(0),
        // Only `oa`, `oe`, `uy` move between the conventions; `cái` does not.
        2 => Some(usize::from(
            style == ToneStyle::Old && has_old_style_placement(syllable),
        )),
        3 => Some(1),
        _ => None,
    }
}
```

Conventions: `snake_case`, `CamelCase` types, no `unwrap`/`expect`/`panic!` outside tests, no `unsafe` outside `viet-ffi`, errors as `Option`/enums rather than strings, comments explain *why* (orthography rules cite the case they protect).

## 7. Testing strategy

| Level | Location | What it proves |
|---|---|---|
| Corpus (golden) | `tests/corpus.rs` | Vietnamese correctness on ≥ 2,000 cases across 4 method/style combinations |
| Unit | `#[cfg(test)]` per module | parser and tone-index edge cases in isolation |
| Property | `tests/invariants.rs` | no panic on arbitrary char streams; `reset` is total; restore never loses a keystroke; output is always NFC |
| Fuzz | `fuzz/` | crash/hang resistance, run manually before releases |
| Bench | `benches/` | p99 keystroke latency < 1 ms (target: < 10 µs) |

Coverage expectation: 100% of branch points in `tone.rs` and `syllable.rs` (these are the bug farms). No coverage target elsewhere.

Corpus policy: a failing case is never deleted or its expectation edited to make CI green. Either the engine is fixed, or the case is reclassified with a comment citing why the expectation was wrong.

## 8. Boundaries

- **Always:** run `cargo fmt`, `clippy -D warnings`, and the full test suite before any commit touching `crates/`; add a corpus case with every behaviour change; keep `viet-core` free of platform types.
- **Ask first:** adding any crate (including dev-dependencies), changing the `Action`/C ABI shape, changing the corpus file format, changing public method names, adding a new method (VNI variants, VIQR).
- **Never:** put fcitx5/GTK/Qt/DBus types in `engine-core`; edit a corpus expectation without a cited source; use `unsafe` outside `viet-ffi`; make the engine read config from disk or env; ship a file whose output is NFD.

## 9. Success criteria

1. `cargo test --workspace` green with ≥ 2,000 corpus cases across the four method/style files.
2. `cargo clippy --workspace --all-targets -- -D warnings` clean.
3. Fuzz target runs 10⁶ random inputs with zero panics and zero hangs.
4. Benchmark: p99 `feed` < 1 ms, no allocation after the first 100 keystrokes (measured with a counting allocator).
5. The exact bug class from the original complaint is representable: `Engine::feed` can express "replace 5 committed chars with 5 new chars" (SurroundingText) and "commit 1 char, consume key" (ForwardKey) without changing the core.
6. `viet-cli type --method telex "vieejt"` prints `việt` on a clean checkout.

## 10. Decisions (2026-11-12)

| # | Decision | Consequence |
|---|---|---|
| D1 | Zero runtime crates; a static table maps the closed set of Vietnamese precomposed characters. `unicode-normalization` is a **dev-dependency** used only as an NFC test oracle. | No shipped dependency, no allocation in the hot path, deterministic output. NFD arriving from outside the engine is normalized at the frontend boundary if ever accepted. |
| D2 | Licence `MIT OR Apache-2.0`. | `LICENSE-MIT` and `LICENSE-APACHE` exist and every `Cargo.toml` inherits `license.workspace = true`. Copyright holder recorded as `beanstalklab`; confirm or replace before the first release. |
| D3 | v1 categories: Telex, Simple Telex, VNI; Unicode NFC only; tone style old **and** new. VIQR, TCVN3, VNI-Windows deferred. | Legacy encodings are conversion features, not input features; they get their own tool if demanded. |
| D4 | `free_tone_mark` ships in v1, default **on**. | The raw-key buffer already supports it; the flag only decides whether a tone key arriving after the coda is honoured. |
| D5 | Product name: **`directtype`** — language-neutral, names the architecture ("direct typing": keystrokes reach the application without a composition buffer). Rejected: `gochuan`/`gotieng` (Vietnamese-only), `gonhanh` (existing macOS IME), `goviet` (ride-hailing brand), `keylang`/`keywrite`/`langkey` (adjacent IME/layout projects), `polytype`/`omnitype`/`keyflow`/`anylang` (unavailable). | Package names: `directtype`, `fcitx5-directtype`, binary `directtype`, `.deb` `directtype`. `directype` is the shorter variant if length matters. Domain and trademark checks remain human work. |

Open question that remains: the corpus source (`tasks/plan.md` Q2).
