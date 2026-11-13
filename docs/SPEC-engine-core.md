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
    pub auto_restore: bool,      // invalid word -> give the raw keystrokes back
    pub spell_check: bool,       // never transform a word that cannot become Vietnamese
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
- A `Boundary` action that gives the raw keystrokes back reports `consumed: false`
  and the whole word as `backspaces` + `commit`, so the frontend rewrites the word
  and then forwards the key it was given (a space still inserts a space).

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

Tone keys in Telex, in the detail T1.4 pinned down:

- A tone key is a letter until it is inside a syllable: `sz → sz`, `telex → telex`,
  and a word that never becomes one syllable keeps every tone key as a letter
  (`ashello → ashello`).
- A later tone key replaces the tone instead of adding a second one: `asf → à`,
  `asfrxj → ạ`.
- The mark is placed once the word is complete, from the syllable §3.4 describes, so
  it moves when the word grows: `toasn → toán` (the mark typed on `toa` ends up on
  `a` after `n` arrives).
- Doubling a tone key undoes it and keeps the letter: `ass → as`, `aass → âs`,
  `toanss → toans`. Shift is not a different key, so `ASs → AS` (§3.7).
- `z` removes the tone and puts the tone key's letter back where it was typed, which
  is the same rule as doubling seen from the other side: `asz → as`, `asdz → asd`,
  `toansz → toans`. With no tone to remove, `z` is a letter (`az → az`).

Letter modifiers in Telex, in the detail T1.2 pinned down:

- Doubling undoes the modifier and keeps the key literal: `aaa → aa`, `ddd → dd`, `ww → w`, `uoww → uow`. The letter that survives keeps the case already on screen (§3.7).
- `w` after `o` horns both letters of the `uo` pair, because one keystroke has to reach `ươ`: `uow → ươ`, `uwow → ươ`.
- `[` and `]` insert `ơ` and `ư` themselves; they are listed as letters above, not as a modification of a preceding letter (`a[` is `aơ`, not `ă`).
  With `auto_restore` on — the default — a word that only reaches an invalid
  spelling this way is given back at the boundary (`a[` → `a[`), which is §3.5.
- A letter that already carries a different mark keeps it and the key stays literal: `awa → ăa`, `oow → ôw`.
- Case follows the letter that carries the mark, which is the first of the pair: `AA → Â`, `aA → â`, `VIEE → VIÊ`. §3.7 has the whole rule, including what an undo does to case.

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
Whether `xt` is a real coda, or `ngh` a real onset, is a spelling question, answered
by §3.6 — the parser itself stays structural, because tone placement must keep
working on words that are not Vietnamese.

### 3.4 Tone placement

Order of rules:

1. If any nucleus vowel carries a quality diacritic (`ă â ê ô ơ ư`), place the tone on it; if several, use the last one that the orthography allows (`ướ`, `uyễ`).
2. Otherwise, if the syllable has a coda, place the tone on the last nucleus vowel (`toán`, `tiếng`).
3. Otherwise (open syllable): one vowel → that vowel; two → first (`cái`, `của`); three → second (`khuỷu`).
4. `ToneStyle::Old` vs `New` changes only the `oa / oe / uy` open-syllable case (`hoà` vs `hòa`, `thuỷ` vs `thủy`). Every other open pair keeps the first vowel under both styles: `cái` and `của` must not move.

`free_tone_mark` decides *when* a tone key is accepted, not where the mark goes:

- On (default): a tone key is a modifier anywhere inside the syllable, and the mark is
  placed from the finished syllable, so `toasn` and `toans` both reach `toán`.
- Off: a tone key is a modifier only after the last letter of the word, which is what
  Telex users call "the mark at the end". `vieetj` is `việt`, `vieejt` keeps the `j` as
  a letter (`viêjt`), and `toasn` keeps its `s` (`toasn`).

Every rule above must be backed by at least three corpus cases, including a counter-example. If implementation and corpus disagree, the corpus wins only when it cites a source in its comment.

### 3.5 Commit and restore

- **Passthrough commit:** a character that needs no transformation is committed
  immediately (`consumed = true`, `commit` = that character) and is never held in
  preedit. This is what makes the no-underline promise structural: the application
  only ever receives committed text, never a composition buffer.
- Word boundary: commit the transformed buffer, clear state, `boundary = true`.
- Backspace during composition removes the last *raw* key, then re-derives the
  transformation from scratch. This is why the engine keeps the raw key sequence
  alongside the transformed text.

`auto_restore` (default on) is decided at the boundary, because that is the first
moment the word is known to be finished:

- Invalid syllable → the frontend deletes the transformed word and receives the raw
  keystrokes exactly as typed (`asd` → `asd`, `asdf` → `asdf`, `oow` → `oow`). The
  boundary key is not consumed: after the rewrite it reaches the application as its
  own keystroke, so a space still inserts a space.
- Valid syllable → nothing to restore; the marks stay (`dduowngf` → `đường`,
  `vieejt` → `việt`).
- Restoring is only exact because the raw keys were kept, so a restore can never
  lose, reorder or duplicate a keystroke — `lib.rs` asserts it over 2,000 generated
  words, comparing a restoring engine against a non-restoring one.

Three guards keep the rewrite away from words that are not mangled text:

| Guard | Why | Example |
|---|---|---|
| shown text equals the keystrokes | nothing was transformed, nothing to undo | `hello`, `telex` |
| no diacritic in the shown text | the word is the keystrokes minus an undone modifier | `aaa` shows `aa`, `ddd` shows `dd`, `aww` shows `aw` |
| no vowel in the shown text | no syllable, no tone, nothing to protect | `dd` shows `đ` |

What survives the guards is a word that carries a mark, has a vowel, and is still
not a syllable. Consequences worth pinning:

- An undo that leaves an invalid spelling is restored like any other invalid word:
  `aass` reaches `âs` (the `s` cancelled the tone) and the boundary gives back
  `aass`. The T1.4 undo rule is what `auto_restore` off shows, and is covered there.
- `test` reaches `tét`, which *is* Vietnamese — `nét`, `mét` and `sét` share its
  rhyme — so no spelling rule may rewrite it. English typing needs the language
  toggle, or `free_tone_mark` off, which leaves the `s` literal because it no longer
  trails the word (`test` → `test`).
- With `auto_restore` off, the boundary keeps the best transformed attempt, which
  for Telex is the letter transformation with every tone key kept literal
  (`asd` → `ád`, `aass` → `âs`, `ashello` → `ashello`).

`spell_check` (default off) is stricter and acts while typing: a transformation the
word can no longer justify is refused, and the raw keys are shown instead. `asd`
therefore never shows `ád` at all, `aaa` stays `aaa` (the `aa` it would become is not
a syllable and cannot become one), and `hello` is untouched — trivially, because
Telex changes nothing in it. The check runs on every keystroke, on that keystroke's
transformation, so a word that starts looking Vietnamese again is transformed again.
Because the whole word is compared, a spelling that is a dead end in the middle can
revert more than the last keystroke: `uwow` shows `ưo` and then `ươ`, and with
`spell_check` on it shows `uwo` in between. The canonical `uow` has no such step.

### 3.6 Spelling: what counts as Vietnamese

`spelling.rs` answers one question — is this letter sequence a syllable? — with
three closed sets:

| Part | Inventory |
|---|---|
| onset | *(empty)*, `b c ch d đ g gh gi h k kh l m n ng ngh nh p ph qu r s t th tr v x` |
| nucleus | `a ă â e ê i o ô ơ u ư y`, then `ai ao au âu ay ây eo êu ia iê iu oa oă oe oi ôi ơi ua uâ uê ui uô uơ uy ưa ưi ưu ươ yê`, then `iêu oai oao oay oeo uây uôi ươi ươu uya uyê uyu yêu` |
| coda | *(empty)*, `c ch m n ng nh p t` |

A word is a syllable when all three parts are listed. Comparison ignores case and
tone but not the quality diacritic (`tét` and `tết` are different syllables, `tét`
and `tẹt` are the same one). `can_become_valid_syllable` is the same test applied to
each part as a prefix, so `ngh` is accepted while `nghe` is still coming.

The tables over-generate on purpose, and the trade-off is not symmetric:

- A word wrongly called invalid is given back as raw keystrokes, which destroys
  diacritics the user asked for: visible, and it loses their work.
- A word wrongly called valid only keeps a transformation they may not want, which
  is what the toggle key and `spell_check` exist for.

So there is no nucleus-to-coda compatibility matrix (`tếc` passes even though
Vietnamese does not spell it), and rare-but-real spellings are all listed. The
inventories come from `BambooEngine/bamboo-core`'s `spelling.go` (MIT), the rule set
behind the most widely used Linux Vietnamese IME; its `firstConsonantSeqs`,
`vowelSeqs` and `lastConsonantSeqs` are the same closed sets. Two deliberate
omissions: `k` is not a coda (it only ends loanwords; Vietnamese spells `ốc`), and
`oo` is not a nucleus (`xoong` is a loanword).

The `viet-core` unit tests cover the tables with real words (`nghiêng`, `khuỷu`,
`rượu`, `nguyễn`) and dead ends (`tes`, `hello`, `awa`), so a table edit that starts
rejecting Vietnamese fails immediately.

### 3.7 Case

The engine never folds case. Four rules produce every mixed-case word:

1. A letter keeps the case of the key that produced it. A modifier marks the letter in
   front of it and does not set that letter's case, so the first letter of the pair
   decides: `Aa → Â` and `aA → â`, `AW → Ă`, `Dd → Đ`, `DD → Đ`, `dD → đ`.
2. A tone key does not set the case of the vowel it marks: `aS` and `AS` both mark a
   lowercase `a` and differ only in the tone key's case, so they are `á` and `Á`. That
   is what keeps a capital where the user typed it inside a word: `Vieejt → Việt`,
   `vIeejt → vIệt`, `VIEEJT → VIỆT`, `vieeJt → việt`.
3. An undo keeps the case of the letter already on screen, so removing a diacritic
   never flips what the user is looking at: `Ư` becomes `W` (`Ww → W`), `Â` becomes `AA`
   (`AaA → AA`). Where the undo leaves a key behind it is the key that undid the mark
   (`aww → aw`, so `AWw → Aw`) or, when the first key only inserted a letter, the key of
   that first press (`Ww → W`, `wW → w`).
4. Shift is not a different key. Doubling ignores case (`ASs → AS`, `AsS → As`), `z`
   removes the tone in either case (`ASZ → AS`), and the letter put back at the tone
   key's position keeps the case it was typed with.

`auto_restore` returns the raw keys, so a restored word keeps its case by construction
(`Asd → Asd`, `ASD → ASD`).

Two of the guarantees are corpus-level properties, not case lists:

- **Case invariance.** Upper-casing the input upper-cases the output, and Title-casing
  the input Title-cases the output. `crates/viet-cli/tests/corpus.rs` derives both twins
  from every case in `tests/corpus`, so a case added later is covered the moment it
  lands and no file needs a hand-written second copy that could drift.
- **Precomposed output.** No output contains a combining mark (U+0300–U+036F). Every
  character the engine inserts comes from the closed table in `unicode.rs`, so a word
  typed without combining marks comes out in NFC. `unicode.rs` has no case-folded
  fallback and no case table of its own: uppercase Vietnamese letters are listed
  explicitly, which is why a missing capital is a visible hole rather than a silent
  behaviour difference.

## 4. Commands

Run from the repository root. `cargo` is user-local on this host, so source
`~/.cargo/env` first; other hosts only need a stable toolchain with clippy.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
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
  src/lib.rs                    public API: Config, Method, ToneStyle, Input, Action, Engine
  src/syllable.rs               onset/nucleus/coda parsing (structure only)
  src/tone.rs                   tone placement rules 3.4
  src/telex.rs                  Telex fold: letter modifiers and tone keys
  src/spelling.rs               closed onset/nucleus/coda inventories, auto_restore guards
  src/unicode.rs                Vietnamese precomposed table, case handling
  src/vni.rs                    planned, T1.7
  tests/invariants.rs           property tests, T2.2
  fuzz/                         cargo-fuzz target, T2.2 (dev only)
crates/viet-cli/src/main.rs
crates/viet-cli/src/corpus.rs   corpus runner behind `viet-cli check`
crates/viet-ffi/
  src/lib.rs
  include/viet.h
tests/corpus/telex-new.tsv       passthrough cases
tests/corpus/telex-letters.tsv   letter modifiers
tests/corpus/telex-tone.tsv      tone keys, new tone style
tests/corpus/telex-tone-old.tsv  tone keys, old tone style
tests/corpus/telex-restore.tsv   auto_restore: invalid words, undone modifiers, limits
tests/corpus/telex-case.tsv      case handling: lower, Title, upper, mixed
tests/corpus/telex-case-old.tsv  case handling with old tone placement
tests/corpus/simple-telex-new.tsv  planned, T1.8
tests/corpus/vni-new.tsv           planned, T1.7
scripts/corpus_tool.py          planned; Python 3.14 stdlib only: sort, dedupe, validate TSV
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
`telex-letters.tsv` runs full Telex with the new style, `telex-tone-old.tsv` runs
tone cases with old placement, and `simple-telex-old.tsv` runs Simple Telex with
that style.

The runner feeds each input character as `Input::Char`, then `Input::Boundary`, and compares the concatenation of `commit` values *after* honouring `backspaces`, for both the character actions and the boundary action: a case can express a modifier that rewrites an earlier character, and `auto_restore` giving the word back. Whitespace inside an input ends the word there, so `asd asd` is two words. Uppercase and mixed case are explicit cases, not implicit behaviour.

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
| Unit | `#[cfg(test)]` per module | parser, tone-index, spelling-inventory and restore-guard edge cases in isolation |
| Property | `tests/invariants.rs` | no panic on arbitrary char streams; `reset` is total; output is always NFC |
| Fuzz | `fuzz/` | crash/hang resistance, run manually before releases |
| Bench | `benches/` | p99 keystroke latency < 1 ms (target: < 10 µs) |

Case is a property too: `crates/viet-cli/tests/corpus.rs` derives the upper-case and
Title-case twin of every corpus case and requires the output to be the twin of the
expectation, and asserts that no output contains a combining mark (the NFC guarantee of
§2.1). Both checks cover cases added later without a second copy of any file.

Restore is a property, not a case list: `viet_core::tests::a_restore_never_loses_or_reorders_a_keystroke`
types 2,000 generated words twice — once with `auto_restore`, once without — and
requires that every difference is exactly the keystrokes. It lives in `lib.rs` until
`tests/invariants.rs` exists in T2.2.

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
| D6 | Spelling validity is a closed-set membership test (onset, nucleus, coda) that over-generates: no nucleus-to-coda matrix, inventories taken from `bamboo-core/spelling.go` (MIT). | A wrongly rejected word would be restored as raw keystrokes, which loses the user's diacritics; accepting too much only keeps a transformation the toggle can undo. Consequence: `test` stays `tét` under the default configuration. |
| D7 | `auto_restore` acts at the boundary; `spell_check` acts while typing. | The boundary is the first moment "not Vietnamese" is final, so the rewrite cannot fight a word that is still growing (`uwow` shows `ưo`, not `uwo`, when only `auto_restore` is on). `spell_check` is the stricter setting and may revert mid-word. |

Open question that remains: the corpus source (`tasks/plan.md` Q2).