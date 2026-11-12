# Task List: `engine-core`

Ordered by dependency. Each task must leave the workspace buildable and the suite green.
Plan: `tasks/plan.md` · Spec: `docs/SPEC-engine-core.md`

---

## T0.1 — Record blocking decisions

**Description:** Settle Q1–Q5 from `tasks/plan.md` and write them into `docs/SPEC-engine-core.md` §10 (replace each question with its answer) and into the Architecture Decisions table. No code.

**Acceptance criteria:**
- [ ] Dependency policy decided (zero crates vs `unicode-normalization`) and recorded
- [ ] Corpus source decided and recorded, including licence if a word list is used
- [ ] Default `tone_style` recorded
- [ ] Project name and licence recorded
- [ ] `git config user.name` / `user.email` set in this repo
- [ ] `docs/PLAN.md` documents table updated (spec status: approved)

**Verification:** human reads the spec diff; no build needed.

**Dependencies:** none
**Files:** `docs/SPEC-engine-core.md`, `docs/PLAN.md`, `tasks/plan.md`
**Scope:** XS

---

## T0.2 — Bootstrap toolchain

**Description:** Install what this host lacks: Rust (rustup, stable, clippy, rustfmt), build tools, and the fcitx5 development headers needed later. Requires `sudo`. Verify package names first — do not guess.

**Acceptance criteria:**
- [x] `rustc --version`, `cargo --version`, `rustfmt --version`, `cargo clippy --version` all succeed
- [x] `pkg-config --modversion Fcitx5Core` succeeds (needed by `ime-addon`, not this module)
- [x] Installed versions recorded in `AGENTS.md` §Verification
- [x] No change to the repository except that record

**Verification:**
```bash
apt-cache search fcitx5 | grep -E 'dev|frontend|config'
rustc --version && cargo --version && cargo clippy --version
pkg-config --modversion Fcitx5Core
```

**Dependencies:** T0.1 (name decision not needed for this, but do not install before the plan is agreed)
**Files:** `AGENTS.md`
**Scope:** S

---

## T0.3 — Scaffold Cargo workspace

**Description:** Create the workspace exactly as laid out in spec §5: `Cargo.toml` (members `crates/*`), pinned `rust-toolchain.toml`, three crates with minimal `lib.rs`/`main.rs`, `README` untouched. No logic.

**Acceptance criteria:**
- [x] `cargo build --workspace` succeeds
- [x] `cargo fmt --all --check` passes
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [x] `crates/viet-ffi/include/viet.h` exists and matches spec §2.2 exactly
- [x] `viet-cli` prints usage and exits `2` when called with no arguments

**Verification:**
```bash
cargo build --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p viet-cli -- ; echo "exit=$?"
```

**Dependencies:** T0.1, T0.2
**Files:** `Cargo.toml`, `rust-toolchain.toml`, `crates/viet-core/Cargo.toml`, `crates/viet-core/src/lib.rs`, `crates/viet-cli/Cargo.toml`, `crates/viet-cli/src/main.rs`, `crates/viet-ffi/Cargo.toml`, `crates/viet-ffi/src/lib.rs`, `crates/viet-ffi/include/viet.h`
**Scope:** M

---

## T0.4 — CI pipeline

**Description:** GitHub Actions workflow running fmt, clippy and tests on push and PR (amd64 only for now).

**Acceptance criteria:**
- [x] Workflow runs on push and PR
- [x] Steps: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- [x] Failing clippy fails the job
- [x] `AGENTS.md` §Verification describes the same commands as CI (single source of truth)

**Verification:** push a commit with a deliberate `fmt` violation; job fails; revert.

**Dependencies:** T0.3
**Files:** `.github/workflows/ci.yml`, `AGENTS.md`
**Scope:** S

### Checkpoint A
- [x] `cargo test --workspace` runs (0 tests) and passes
- [ ] CI green on `main` (needs the first push to a remote)
- [ ] Human review before Phase 1

---

## T1.1 — Buffer, boundary, passthrough, corpus runner

**Description:** The vertical slice that proves the pipeline. Characters are committed immediately (never held in preedit), the raw word is tracked, and `Input::Boundary` ends the word. Implement `viet-cli type` and `viet-cli check` with exit codes `0/1/2` and a readable diff (`expected` / `got` / line number).

**Acceptance criteria:**
- [x] `viet-cli type --method telex "abc def"` prints `abc def`
- [x] Each character is committed immediately (`consumed = true`, `commit` = that character); `Boundary` clears the word and reports `boundary = true` (refined in spec §3.5: input is never held back, which is what removes the underline)
- [x] `viet-cli check` parses TSV, skips `#` comments, exits `1` on mismatch, `2` on malformed or unreadable corpus
- [x] Empty corpus directory → exit `0`
- [x] `tests/corpus/telex-new.tsv` has 10 passthrough cases
- [x] Reallocation test: the commit buffer's capacity is stable after warmup

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- type --method telex "abc def"
cargo run -p viet-cli -- check tests/corpus
```

**Dependencies:** T0.3
**Files:** `crates/viet-core/src/lib.rs`, `crates/viet-cli/src/lib.rs`, `crates/viet-cli/src/corpus.rs`, `crates/viet-cli/src/main.rs`, `crates/viet-cli/tests/corpus.rs`, `tests/corpus/telex-new.tsv`, `.github/workflows/ci.yml`
**Scope:** M

---

## T1.2 — Telex letter modifiers

**Description:** Implement the letter transformations from spec §3.2 for Telex: `aa/ee/oo`, `w`, `aw/ow/uw`, `dd`, `[`, `]`, with doubling to undo (`aaa → aa`, `ddd → dd`, `ww → w`). Pure mapping over the buffer; no tones yet.

**Acceptance criteria:**
- [ ] `vieejt`-class vowel runs transform correctly
- [ ] Doubling undoes the modifier and keeps the literal key
- [ ] All ~30 mapping cases in `tests/corpus/telex-letters.tsv` pass
- [ ] Uppercase inputs map to uppercase outputs (`AA → Â`), lowercase to lowercase
- [ ] No allocation regression

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
```

**Dependencies:** T1.1, T0.1 (corpus source decision, Q2)
**Files:** `crates/viet-core/src/telex.rs`, `crates/viet-core/src/unicode.rs`, `tests/corpus/telex-letters.tsv`
**Scope:** M

---

## T1.3 — Syllable parser and tone index

**Description:** Parse the buffer into `onset · medial · nucleus · coda` and compute the tone position per spec §3.4. This task produces the parser and `tone_index()` only; it does not apply tones yet.

**Acceptance criteria:**
- [ ] `qu` and `gi` treated as onsets (`quà`, `quý`, `già`, `gì`)
- [ ] Nucleus of length 1/2/3 parsed correctly with and without coda
- [ ] `tone_index` returns the spec §3.4 position for: `toán`, `tiếng`, `cái`, `của`, `khuỷu`, `ướ`
- [ ] `ToneStyle::Old/New` differ only for `oa/oe/uy` open syllables (`hoà` vs `hòa`)
- [ ] Invalid sequences return `None`, never panic
- [ ] 100% branch coverage in `tone.rs` and `syllable.rs`

**Verification:**
```bash
cargo test -p viet-core syllable:: tone::
cargo test --workspace
```

**Dependencies:** T1.1
**Files:** `crates/viet-core/src/syllable.rs`, `crates/viet-core/src/tone.rs`, `crates/viet-core/tests/invariants.rs`
**Scope:** M

### Checkpoint B
- [ ] `viet-cli type --method telex "vieejt"` prints `việt`
- [ ] Parser and tone-index tests green, including `qu`/`gi`
- [ ] Human review before tone application

---

## T1.4 — Telex tone keys, removal, free tone mark

**Description:** Apply `s/f/r/x/j` to the position from T1.3; `z` removes a tone; doubling a tone key removes it and keeps the letter (`ass → as`). `free_tone_mark` allows a tone key typed at any position to reposition.

**Acceptance criteria:**
- [ ] Five tone keys produce the correct precomposed character
- [ ] Tone on the right vowel for all T1.3 cases
- [ ] `z` removes a tone, `ass → as`, `asz → as`
- [ ] `free_tone_mark` on: `vieejt` and `viejte`-style orderings both yield `việt`
- [ ] `telex-tone.tsv` ≥ 100 cases passing

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
```

**Dependencies:** T1.3
**Files:** `crates/viet-core/src/telex.rs`, `crates/viet-core/src/tone.rs`, `tests/corpus/telex-tone.tsv`
**Scope:** M

---

## T1.5 — Auto-restore and spell-check

**Description:** Invalid syllable + `auto_restore` → commit raw keystrokes. `spell_check` on → leave non-Vietnamese-looking words untouched. Applies to both Telex and the shared buffer.

**Acceptance criteria:**
- [ ] `asd` with `auto_restore` commits `asd` (raw), not a half-transformed string
- [ ] Restore never loses or reorders a keystroke (property test)
- [ ] `spell_check` on: `hello` passes through unchanged
- [ ] `spell_check` off: engine commits its best transformed attempt
- [ ] Both switches independent; 4-way config matrix tested

**Verification:**
```bash
cargo test -p viet-core auto_restore
cargo test --workspace
```

**Dependencies:** T1.4
**Files:** `crates/viet-core/src/lib.rs`, `crates/viet-core/src/syllable.rs`, `tests/corpus/restore.tsv`
**Scope:** M

---

## T1.6 — Case handling

**Description:** Correct case for lowercase, UPPERCASE, Title and mixed input, including Vietnamese diacritics (`Đ`, `Ư`, `Ế`).

**Acceptance criteria:**
- [ ] `VIEEJT → VIỆT`, `Vieejt → Việt`, `vIeejt → vIệt` (preserve position of capitals)
- [ ] `DD → Đ`, `dd → đ`
- [ ] No output contains a combining mark (NFC assertion over the whole corpus)
- [ ] Every corpus case also has a Title-case twin where meaningful

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
```

**Dependencies:** T1.4
**Files:** `crates/viet-core/src/unicode.rs`, `crates/viet-core/tests/corpus.rs`, `tests/corpus/telex-case.tsv`
**Scope:** S

### Checkpoint C
- [ ] `telex-new.tsv` and `telex-old.tsv` green
- [ ] Every spec §3.4 rule has ≥ 3 cases plus a counter-example
- [ ] Human review before VNI

---

## T1.7 — VNI method

**Description:** VNI key mapping from spec §3.2 (`1..5`, `0`, `6`, `7`, `8`, `9`) on the shared syllable/tone core. `tone.rs` must not gain VNI-specific branches.

**Acceptance criteria:**
- [ ] `viet65t` → `việt`-equivalent VNI case set passes
- [ ] `d9 → đ`, `a8 → ă`, `o7 → ơ`, `u7 → ư`, `a6/e6/o6 → â/ê/ô`
- [ ] `0` removes tone; doubled modifier key undoes
- [ ] `vni-new.tsv` ≥ 200 cases
- [ ] Diff of `tone.rs` against Checkpoint C is empty

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
git diff main -- crates/viet-core/src/tone.rs
```

**Dependencies:** T1.5, T1.6
**Files:** `crates/viet-core/src/vni.rs`, `crates/viet-core/src/input.rs`, `tests/corpus/vni-new.tsv`
**Scope:** M

---

## T1.8 — SimpleTelex method

**Description:** Restricted Telex variant: only `aa/ee/oo`, `w`, `dd`.

**Acceptance criteria:**
- [ ] `aw` stays `aw` in SimpleTelex (not `ă`), `aa → â`
- [ ] `simple-telex-new.tsv` ≥ 50 cases
- [ ] Method selection via `Config` only; no duplicated tone logic

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
```

**Dependencies:** T1.7
**Files:** `crates/viet-core/src/telex.rs`, `crates/viet-core/src/config.rs`, `tests/corpus/simple-telex-new.tsv`
**Scope:** S

### Checkpoint D
- [ ] All four corpus files green, total ≥ 2,000 cases
- [ ] `viet-cli check` exits `0`
- [ ] Human review before robustness work

---

## T2.1 — Backspace and mid-composition editing

**Description:** Backspace removes the last *raw* key and re-derives the transformed word from scratch. Word boundary on backspace-at-empty-buffer is defined explicitly.

**Acceptance criteria:**
- [ ] Every prefix of every corpus case, followed by Backspace at the end, yields the corpus expectation of that prefix
- [ ] Backspace on empty buffer returns `consumed: false`
- [ ] Backspace after commit does not recall the previous word
- [ ] Property test: for random input + backspaces, final state equals the engine state of the surviving raw sequence

**Verification:**
```bash
cargo test -p viet-core backspace
cargo test --workspace
```

**Dependencies:** T1.8
**Files:** `crates/viet-core/src/buffer.rs`, `crates/viet-core/src/lib.rs`, `crates/viet-core/tests/invariants.rs`
**Scope:** M

---

## T2.2 — Property tests and fuzz target

**Description:** `proptest` (dev-dependency, requires T0.1 approval) for invariants; `cargo-fuzz` target for crash resistance.

**Acceptance criteria:**
- [ ] Invariants: no panic, `reset` is total, restore preserves keystrokes, output is NFC, no allocation in `feed` after warmup
- [ ] Fuzz target builds and runs 10⁶ inputs with zero panics/hangs
- [ ] Findings, if any, become regression cases in `tests/corpus/regressions.tsv`

**Verification:**
```bash
cargo test --workspace --release
cargo +nightly fuzz run engine -- -runs=1000000
```

**Dependencies:** T2.1, T0.1
**Files:** `crates/viet-core/tests/invariants.rs`, `crates/viet-core/fuzz/Cargo.toml`, `crates/viet-core/fuzz/fuzz_targets/engine.rs`, `tests/corpus/regressions.tsv`
**Scope:** M

---

## T2.3 — Benchmarks and allocation check

**Description:** `criterion` bench for `feed` across the corpus; a counting-allocator test asserting steady-state zero allocations.

**Acceptance criteria:**
- [ ] `cargo bench -p viet-core` reports p99 for a realistic key stream
- [ ] p99 < 1 ms (target < 10 µs) recorded in `AGENTS.md` §Verification
- [ ] Allocation test fails if a `Vec`/`String` grows in steady state

**Verification:**
```bash
cargo bench -p viet-core
cargo test -p viet-core --release allocations
```

**Dependencies:** T2.2
**Files:** `crates/viet-core/benches/feed.rs`, `crates/viet-core/tests/allocations.rs`, `AGENTS.md`
**Scope:** S

### Checkpoint E
- [ ] Fuzz clean at 10⁶ runs
- [ ] p99 latency recorded and within target
- [ ] Human review before FFI

---

## T3.1 — `viet-ffi`: C ABI

**Description:** Implement spec §2.2 exactly: opaque engine pointer, `viet_action_t`, borrowed string spans, `catch_unwind` at the boundary, `#[no_mangle] extern "C"`, `staticlib` + `cdylib`. No ownership transfer.

**Acceptance criteria:**
- [ ] Header and implementation agree; a Rust test and a C test call the same functions
- [ ] A forced panic inside the core returns a safe no-op action instead of unwinding
- [ ] Strings are valid UTF-8 and NUL-terminated only in the header's documented way (length-based, not NUL-truncated)
- [ ] `cargo build -p viet-ffi --release` produces `libviet.a` and `libviet.so`
- [ ] No `unsafe` outside this crate

**Verification:**
```bash
cargo build -p viet-ffi --release
cargo test -p viet-ffi
nm -D target/release/libviet.so | grep viet_engine
```

**Dependencies:** T2.3
**Files:** `crates/viet-ffi/src/lib.rs`, `crates/viet-ffi/include/viet.h`, `crates/viet-ffi/Cargo.toml`
**Scope:** M

---

## T3.2 — C smoke test linked against the staticlib

**Description:** A ~40-line C program that compiles the corpus path: create engine, feed `vieejt`, assert `việt`, free. Built in CI with `cc`, proving the header is usable from C++ too.

**Acceptance criteria:**
- [ ] `cc -std=c11 -I crates/viet-ffi/include tests/ffi_smoke.c target/release/libviet.a -o /tmp/ffi_smoke` succeeds
- [ ] Running it exits `0` and prints `ok`
- [ ] Same source compiles as C++17 (`c++ -std=c++17`) with no warnings
- [ ] CI job added

**Verification:**
```bash
cc -std=c11 -I crates/viet-ffi/include tests/ffi_smoke.c target/release/libviet.a -o /tmp/ffi_smoke && /tmp/ffi_smoke
c++ -std=c++17 -Wall -Wextra -I crates/viet-ffi/include tests/ffi_smoke.c target/release/libviet.a -o /tmp/ffi_smoke_cpp
```

**Dependencies:** T3.1
**Files:** `tests/ffi_smoke.c`, `.github/workflows/ci.yml`
**Scope:** S

### Checkpoint F — module done
- [ ] Spec §9 success criteria 1–6 all demonstrably met
- [ ] `docs/SPEC-engine-core.md` status set to `implemented`
- [ ] `docs/PLAN.md` documents table updated
- [ ] Human reviews before starting `ime-addon`
