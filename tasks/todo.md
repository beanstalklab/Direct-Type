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
- [x] `vieejt`-class vowel runs transform correctly (`viee → viê`; the tone key needs T1.4)
- [x] Doubling undoes the modifier and keeps the literal key (`aaa → aa`, `ddd → dd`, `ww → w`, `uoww → uow`)
- [x] All mapping cases in `tests/corpus/telex-letters.tsv` pass (56 cases, including the passthrough file)
- [x] Uppercase inputs map to uppercase outputs (`AA → Â`), lowercase to lowercase
- [x] No allocation regression (buffer capacities stable after warmup)
- [x] Corpus runner honours `backspaces`, so a case can express a rewrite of an earlier character

**Decisions recorded in this task:** `w` after `o` horns both letters of the `uo`
pair while the word goes on (`uowng → ương`), because `ương` is unreachable in one
keystroke otherwise, while a word that ends at the `w` keeps `uơ` (`thuow → thuơ`, so
`thuở`, `quơ` and `huơ` stay spellable — corrected in T1.7, see below); `[` and
`]` insert `ơ`/`ư` directly and do not modify a preceding letter; a letter carrying
a different mark keeps it and the key stays literal (`awa → ăa`). Spec §3.2 and the
corpus naming convention in §5 were updated in the same change.

**Corpus source (Q2), for now:** hand-written cases derived from spec §3.2, no
imported word list and no third-party oracle. Widening with an oracle stays open.

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

**Description:** Parse the buffer into `onset · nucleus · coda` and compute the tone position per spec §3.4. This task produces the parser and `tone_index()` only; it does not apply tones yet. The medial is folded into the nucleus — see the decision note below.

**Acceptance criteria:**
- [x] `qu` and `gi` treated as onsets (`qua`, `quy`, `gia`, `giư`); `gi` only when a vowel follows, so `gì` keeps `i` in the nucleus
- [x] Nucleus of length 1/2/3 parsed correctly with and without coda
- [x] `tone_index` returns the spec §3.4 position for `toán`, `tiếng`, `cái`, `của`, `khuỷu`, `ướ` (tested through their untoned spellings, since tone application is T1.4)
- [x] `ToneStyle::Old/New` differ only for `oa/oe/uy` open syllables (`hoà` vs `hòa`), with `cái`/`của` as counter-examples
- [x] Invalid sequences return `None`, never panic
- [x] Every branch of `tone.rs` and `syllable.rs` exercised by a test (no coverage tool is installed; installing one is an ask-first item)

**Decisions recorded in this task:** no separate medial field — the glide belongs
to the nucleus, because otherwise `hoa` would have one nucleus vowel and `khuỷu`
two, and both tone placements would be wrong (spec §3.3 updated). The style rule is
restricted to `oa`/`oe`/`uy`; the earlier spec sample applied it to every two-vowel
open syllable, which would have produced `caí` (spec §3.4 and §6 corrected). The
parser is structural: onset/coda *validity* is the spelling question T1.5 answers.

**Verification:**
```bash
cargo test -p viet-core syllable:: tone::
cargo test --workspace
```

**Dependencies:** T1.1
**Files:** `crates/viet-core/src/syllable.rs`, `crates/viet-core/src/tone.rs`, `crates/viet-core/tests/invariants.rs`
**Scope:** M

### Checkpoint B
- [x] `viet-cli type --method telex "vieejt"` prints `việt`
- [x] Parser and tone-index tests green, including `qu`/`gi`
- [ ] Human review before tone application

---

## T1.4 — Telex tone keys, removal, free tone mark

**Description:** Apply `s/f/r/x/j` to the position from T1.3; `z` removes a tone; doubling a tone key removes it and keeps the letter (`ass → as`). `free_tone_mark` allows a tone key typed at any position to reposition.

**Acceptance criteria:**
- [x] Five tone keys produce the correct precomposed character
- [x] Tone on the right vowel for all T1.3 cases
- [x] `z` removes a tone, `ass → as`, `asz → as`
- [x] `free_tone_mark` on: `vieejt` and `vieetj` both yield `việt` (the task text said
      `viejte`-style, which cannot reach `việt`; the working pair is the mark before
      the coda and the mark after the last letter)
- [x] `telex-tone.tsv` ≥ 100 cases passing (152 cases, plus 22 in `telex-tone-old.tsv`)

**Decisions recorded in this task:**

- A tone key is a letter until the word can take a tone (`sz → sz`), and a word that
  never becomes one syllable keeps every tone key literal (`ashello → ashello`), so
  no keystroke is lost before `auto_restore` exists.
- The mark is placed from the *finished* syllable, not from the text at the moment the
  key was typed. That is what makes `toasn → toán` work with `free_tone_mark` on and
  what makes the mark follow a syllable that grows (`toa` → `toan`).
- Removing a tone (doubling or `z`) puts the tone key's letter back at the position
  where it was typed, so `asdz → asd`, not `asd`-reordered or `ads`. `z` with no tone
  is the letter `z` (`az → az`).
- `free_tone_mark` off means "the mark must follow the last letter of the word":
  `vieetj → việt`, `vieejt → viêjt`. It is a timing rule, not a placement rule, so
  `tone.rs` is unchanged by it.
- A later tone key replaces the tone (`asf → à`); it never stacks `á` + `à`.
- VNI and SimpleTelex still pass their keys through unchanged; their tone logic waits
  for T1.7/T1.8, and `tone.rs` gained no method-specific branch.
- Corpus source: hand-written cases derived from spec §3.2/§3.4, same policy as T1.2 —
  no imported word list, no third-party oracle.

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
```

**Dependencies:** T1.3
**Files:** `crates/viet-core/src/telex.rs`, `crates/viet-core/src/tone.rs`, `crates/viet-core/src/syllable.rs`, `crates/viet-core/src/unicode.rs`, `crates/viet-core/src/lib.rs`, `tests/corpus/telex-tone.tsv`, `tests/corpus/telex-tone-old.tsv`
**Scope:** M

---

## T1.5 — Auto-restore and spell-check

**Description:** Invalid syllable + `auto_restore` → commit raw keystrokes. `spell_check` on → leave non-Vietnamese-looking words untouched. Applies to both Telex and the shared buffer.

**Acceptance criteria:**
- [x] `asd` with `auto_restore` commits `asd` (raw), not a half-transformed string
- [x] Restore never loses or reorders a keystroke (property test)
- [x] `spell_check` on: `hello` passes through unchanged
- [x] `spell_check` off: engine commits its best transformed attempt
- [x] Both switches independent; 4-way config matrix tested
- [x] A word that is Vietnamese keeps every mark (`dduowngf` → `đường`, `vieejt` → `việt`)

**Decisions recorded in this task:**

- Spelling validity is a closed-set membership test on onset, nucleus and coda, with
  the inventories taken from `BambooEngine/bamboo-core` (`spelling.go`, MIT). The
  tables over-generate on purpose: no nucleus-to-coda matrix, and rare-but-real
  spellings are listed. A word wrongly called invalid would be restored as raw keys,
  which destroys the user's diacritics; a word wrongly called valid only keeps a
  transformation the toggle key can undo. See spec §3.6 and decision D6.
- `auto_restore` acts at the boundary, `spell_check` acts while typing (D7). The
  boundary is the first moment "not Vietnamese" is final, so `uwow` shows `ưo` while
  it is still growing; `spell_check` is the stricter switch and may revert mid-word.
- Three guards keep the restore away from words that are not mangled text: the shown
  text equals the keystrokes, or has no diacritic, or has no vowel. That protects
  `hello`, the undo artefacts (`aaa` → `aa`, `ddd` → `dd`, `aww` → `aw`) and `đ`
  from `dd`.
- The test text said `test` → `test`; that needs a source correction rather than an
  engine change. `tét` is real Vietnamese (`nét`, `mét`, `sét` share its rhyme), so a
  rule that only asks "is this a syllable?" must keep it. `test` → `test` happens with
  `free_tone_mark` off, and English typing needs the toggle key. Pinned as a corpus
  case so a later weakening of the tables fails there.
- `aass` → `âs` is the one T1.4 case `auto_restore` changes: the undone tone leaves a
  word no syllable spells, so the boundary gives `aass` back. The T1.4 rule is pinned
  with `auto_restore` off.
- The restoring boundary action reports `consumed: false` with the whole word as
  `backspaces` + `commit`, so a space still inserts a space after the rewrite. The
  engine drops its view of the shown text at that moment, otherwise the next word
  would report deletions against text the frontend has already replaced.

**Verification:**
```bash
cargo test -p viet-core auto_restore
cargo test --workspace
```

**Dependencies:** T1.4
**Files:** `crates/viet-core/src/lib.rs`, `crates/viet-core/src/spelling.rs`,
`crates/viet-core/src/syllable.rs`, `crates/viet-core/src/unicode.rs`,
`crates/viet-cli/src/corpus.rs`, `tests/corpus/telex-restore.tsv`
**Scope:** M

---

## T1.6 — Case handling

**Description:** Correct case for lowercase, UPPERCASE, Title and mixed input, including Vietnamese diacritics (`Đ`, `Ư`, `Ế`).

**Acceptance criteria:**
- [x] `VIEEJT → VIỆT`, `Vieejt → Việt`, `vIeejt → vIệt` (preserve position of capitals)
- [x] `DD → Đ`, `dd → đ`
- [x] No output contains a combining mark (NFC assertion over the whole corpus)
- [x] Every corpus case also has a Title-case twin where meaningful

**Decisions recorded in this task:**

- The "Title-case twin" criterion is a generated property, not a second copy of every
  file: `crates/viet-cli/tests/corpus.rs` derives the upper-case and the Title-case twin
  of every case in `tests/corpus`, requires the output to be the twin of the
  expectation, and asserts that at least 100 twins were checked so the property cannot
  pass vacuously. A case added later is covered without editing another file.
- That property found two real bugs, both from treating shift as a different key.
  Doubling a tone key compared characters case-sensitively, so `ASs` re-applied the
  acute instead of undoing it (now `ASs → AS`). And undoing the inserted `ư` of `w`
  replaced it with the *current* key, so `Ww` flipped the capital to lowercase on
  screen (now `Ww → W` — an undo keeps the case of the letter already there).
- The NFC criterion is checked as "no output character is a combining mark
  (U+0300–U+036F)", which is exactly what a decomposed Vietnamese letter is made of, so
  it needs no Unicode dependency. It runs over the whole corpus and over 500 generated
  words in `viet-core`.
- No `unicode.rs` change was needed: the table already lists uppercase rows explicitly
  and has no case-folded fallback, so case was a rule question, not a table question.
  `crates/viet-core/tests/corpus.rs` from the task text does not exist; the corpus
  integration test belongs to the crate that owns the runner, so it lives in
  `crates/viet-cli/tests/corpus.rs`.
- `viet-cli::corpus` gained `case_files`, `read_cases` and `configuration_from_filename`
  as public functions, so the integration test reads the corpus with the runner's own
  parser instead of a copy of it.

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
```

**Dependencies:** T1.4
**Files:** `crates/viet-core/src/telex.rs`, `crates/viet-core/src/lib.rs`,
`crates/viet-cli/src/corpus.rs`, `crates/viet-cli/tests/corpus.rs`,
`tests/corpus/telex-case.tsv`, `tests/corpus/telex-case-old.tsv`
**Scope:** S

### Checkpoint C
- [x] `telex-new.tsv` green, and the old-style files (`telex-tone-old.tsv`,
      `telex-case-old.tsv`) green with old placement
- [x] Every spec §3.4 rule has ≥ 3 cases plus a counter-example
- [ ] Human review before VNI

---

## T1.7 — VNI method

**Description:** VNI key mapping from spec §3.2 (`1..5`, `0`, `6`, `7`, `8`, `9`) on the shared syllable/tone core. `tone.rs` must not gain VNI-specific branches.

**Acceptance criteria:**
- [x] `vie65t` → `việt`-equivalent VNI case set passes (the task text said `viet65t`;
      `6` after `t` marks nothing, so the working word is `vie65t` or `vie6t5`)
- [x] `d9 → đ`, `a8 → ă`, `o7 → ơ`, `u7 → ư`, `a6/e6/o6 → â/ê/ô`
- [x] `0` removes tone; doubled modifier key undoes
- [x] `vni-new.tsv` ≥ 200 cases (204, plus 17 in `vni-old.tsv`)
- [x] Diff of `tone.rs` against Checkpoint C is empty

**Decisions recorded in this task:**

- The fold moved to `input.rs` behind a `RulePack` trait (`refactor(core)` commit), so
  VNI is a rule pack and not a second copy of the tone state machine. `tone.rs` and
  `syllable.rs` are untouched, which is what the last criterion asks for.
- VNI key table from `fcitx5-bamboo`'s `input_method_def.go` (MIT): `0` XoaDauThanh,
  `6` `AEO_ÂÊÔ`, `7` `UO_ƯƠ`, `8` `A_Ă`, `9` `D_Đ`. A digit that cannot act stays
  visible (`n6`, `a67`), because a keystroke that disappears is worse than one on
  screen.
- `0` removes the tone and drops the tone digit (`a10 → a`), unlike Telex's `asz → as`.
  A tone letter is plausible text to leave behind; a digit is noise. Doubling keeps one
  digit (`a11 → a1`) because the user typed it again. The `RulePack` trait carries this
  as one associated constant rather than a branch in the fold.
- Writing the VNI `7` rule exposed a defect in T1.2's Telex rule: an eager `uow → ươ`
  made `thuở`, `quơ` and `huơ` untypable. Fixed first, as its own commit
  (`fix(core): a word ending at w keeps the uơ rhyme`), with `fcitx5-bamboo`'s
  `regUOhTail` lookahead as the source.
- VNI and Telex keys do not leak: `w`, `[`, `]`, `s`, `z` are ordinary letters in VNI,
  and `1`–`9` are ordinary characters in Telex. Corpus cases pin both directions.

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
git diff main -- crates/viet-core/src/tone.rs
```

**Dependencies:** T1.5, T1.6
**Files:** `crates/viet-core/src/input.rs`, `crates/viet-core/src/vni.rs`,
`crates/viet-core/src/lib.rs`, `tests/corpus/vni-new.tsv`, `tests/corpus/vni-old.tsv`
**Scope:** M

---

## T1.8 — SimpleTelex method

**Description:** Restricted Telex variant: only `aa/ee/oo`, `w`, `dd`.

**Acceptance criteria:**
- [x] `aw` stays `aw` in SimpleTelex (not `ă`), `aa → â`
- [x] `simple-telex-new.tsv` ≥ 50 cases (52)
- [x] Method selection via `Config` only; no duplicated tone logic

**Decisions recorded in this task:**

- Simple Telex is a third `RulePack` in `telex.rs` over the shared fold, so it inherits
  the tone keys, the undo rules, `auto_restore` and `spell_check` without a line of its
  own tone code. `tone.rs` is still unchanged.
- Without the horn and breve shortcuts the method cannot build `ă` or `ơ`, so it is for
  a keyboard that has those letters on its layout: they pass through and the pack adds
  the circumflex, the stroke, the `ư` of `w` and the tones. Words that need the missing
  shortcuts (`dduowng`, `thuong`) are given back by `auto_restore`, and the corpus
  pins that.
- `w` produces `ư` only while the word is still an onset (`w`, `hw`, `thw`). After a
  vowel it is a letter, so the `aw → aw` criterion holds with `auto_restore` off too,
  instead of relying on a later restore to hide `aư`.
- `config.rs` from the task's file list does not exist: the method is selected through
  `Config::method` in `lib.rs`, which is where every method is chosen.

**Verification:**
```bash
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
```

**Dependencies:** T1.7
**Files:** `crates/viet-core/src/telex.rs`, `crates/viet-core/src/lib.rs`,
`tests/corpus/simple-telex-new.tsv`
**Scope:** S

### Checkpoint D
- [x] All corpus files green, `viet-cli check` exits `0` (630 cases across eight files)
- [ ] Total ≥ 2,000 cases — blocked on Q2 (corpus source)
- [ ] Human review before robustness work

---

## T2.1 — Backspace and mid-composition editing

**Description:** Backspace removes the last *raw* key and re-derives the transformed word from scratch. Word boundary on backspace-at-empty-buffer is defined explicitly.

**Acceptance criteria:**
- [x] Every prefix of every corpus case, followed by Backspace at the end, yields the corpus expectation of that prefix
- [x] Backspace on empty buffer returns `consumed: false`
- [x] Backspace after commit does not recall the previous word
- [x] Property test: for random input + backspaces, final state equals the engine state of the surviving raw sequence

**Decisions recorded in this task:**

- No engine change was needed: `Input::Backspace` already popped the raw key and
  re-derived the whole word, which is why it can undo a modifier that shaped an earlier
  letter and can bring a removed tone back. T2.1 is the guarantee, in two properties and
  seven unit tests.
- "Every prefix of every corpus case" is implemented as: type the prefix, delete, end the
  word, and compare with typing the shorter prefix and ending the word. Ending both keeps
  `auto_restore` comparable — a prefix that ends on a space has already finished its word
  and been restored, and comparing that against a live shorter word would compare a
  finished word with an unfinished one.
- A forwarded backspace is the application's, not the engine's: the test helper models the
  frontend deleting the character before the caret, which is what a real frontend does and
  what makes a delete after a boundary observable at all.
- The property was checked against a deliberate mutation (forgetting to re-derive after
  the pop) and failed on `aa`, so it is not a test that passes for free.
- `crates/viet-core/src/buffer.rs` from the task's file list was not created: the raw keys
  are one `String` inside `Engine`, and wrapping it in a module would add a layer with no
  second reason to change. Where the buffer needs a policy — a cap on word length for
  T2.2 — that belongs in the engine next to the other state, not in a wrapper.

**Verification:**
```bash
cargo test -p viet-core backspace
cargo test --workspace
```

**Dependencies:** T1.8
**Files:** `crates/viet-core/src/lib.rs`, `crates/viet-cli/tests/corpus.rs`
**Scope:** M

---

## T2.2 — Property tests and fuzz target

**Description:** `proptest` (dev-dependency, requires T0.1 approval) for invariants; `cargo-fuzz` target for crash resistance.

**Acceptance criteria:**
- [x] Invariants: no panic, `reset` is total, restore preserves keystrokes, output is NFC
- [ ] Invariant: no allocation in `feed` after warmup — T2.3 owns the counting allocator;
      `buffers_stabilize_after_warmup` in `lib.rs` already pins stable capacities
- [ ] Fuzz target builds and runs 10⁶ inputs with zero panics/hangs — **deferred**: needs
      `cargo-fuzz` and `libfuzzer-sys` as dev tooling, which the user has not approved yet
- [x] Findings, if any, become regression cases in `tests/corpus/regressions.tsv` (none
      so far: the invariants and the table check passed on the first run)

**Decisions recorded in this task:**

- No `proptest`: the generator is thirty lines in each test file, so the repository keeps
  its zero-runtime-and-dev-dependency promise without waiting on a T0.1 decision. A
  seeded generator is also reproducible in a bug report, which is what matters for a
  property that fails occasionally.
- The properties are split by what they can see. `tests/invariants.rs` uses the public API
  only — every configuration (three methods × two tone styles × three switches), hostile
  input (controls, combining marks, emoji, a lone U+10FFFF), `reset` totality, a forwarded
  boundary, `preedit` always empty, and no delete reaching past the word. The properties
  that read engine state (restore exactness, delete re-derivation) stay in `lib.rs` and
  the corpus test.
- Each property was checked against a deliberate mutation (a no-op `reset`; forgetting to
  re-derive after a backspace) and failed, so none of them pass for free.
- The word-length cap the raw buffer lacked is in: 64 raw keys, after which the word ends
  where it stands and the next key starts a new one, in one action with no deletions. An
  over-long word is never restored — it never reached a boundary, and nothing a person
  types comes close.
- The NFC requirement gained its oracle: `scripts/verify_unicode_table.py` checks all 144
  entries against Unicode canonical decomposition with Python's `unicodedata`, and CI runs
  it. `unicode.rs` has claimed this script existed since T1.1; it did not, which is now
  fixed rather than restated. Canonical ordering matters — the dot below (combining class
  220) precedes the circumflex and breve (230) in `ặ` — so the script normalizes the
  expected string instead of hard-coding an order.

**Verification:**
```bash
cargo test -p viet-core --test invariants
cargo test --workspace
python3 scripts/verify_unicode_table.py
```

**Dependencies:** T2.1, T0.1
**Files:** `crates/viet-core/tests/invariants.rs`, `scripts/verify_unicode_table.py`,
`crates/viet-core/src/lib.rs`, `.github/workflows/ci.yml`;
`crates/viet-core/fuzz/*` deferred
**Scope:** M

---

## T2.3 — Benchmarks and allocation check

**Description:** p99 `feed` latency under 1 ms with a benchmark, and a counting allocator that proves `feed` allocates nothing after warmup.

**Acceptance criteria:**
- [x] `feed` allocates nothing after warmup, measured with a counting allocator
- [ ] p99 `feed` < 1 ms — **deferred**: a stable `cargo bench` needs `criterion` (or nightly
      `#[bench]`), which the user has not approved yet
- [ ] No allocation in the benchmark's steady state (same tooling question)

**Decisions recorded in this task:**

- The counting allocator is `crates/viet-core/tests/allocations.rs`, a separate test binary
  with exactly one test so nothing else allocates during the measurement. It wraps
  `System`, counts `alloc` and `realloc`, and is the only `unsafe` outside `viet-ffi`
  (spec §8, decision D8). The measurement covers all three methods and the corpus' hardest
  words, plus a word long enough to cross the 64-key cap, so the new cap path is covered
  too: zero allocations after warmup, in debug and in release.
- The test was checked against a one-line mutation that allocates per keystroke and failed
  with 14,100 allocations, so it is not a test that passes for free.
- The capacity-stability test in `lib.rs` stays: it is the cheap proxy, and the counting
  allocator is the real measurement.

**Verification:**
```bash
cargo test -p viet-core --test allocations
cargo test --release -p viet-core --test allocations
```

**Dependencies:** T2.2
**Files:** `crates/viet-core/tests/allocations.rs`, `docs/SPEC-engine-core.md` (D8)
**Scope:** M

---

## T3.1 — `viet-ffi`: C ABI

**Description:** Implement spec §2.2 exactly: opaque engine pointer, `viet_action_t`, borrowed string spans, `catch_unwind` at the boundary, `#[no_mangle] extern "C"`, `staticlib` + `cdylib`. No ownership transfer.

**Acceptance criteria:**
- [x] Header and implementation agree; a Rust test and a C test call the same functions
- [x] A forced panic inside the core returns a safe no-op action instead of unwinding
- [x] Strings are valid UTF-8 and length-delimited in bytes, never NUL-truncated
- [x] `cargo build -p viet-ffi --release` produces `libviet.a` and `libviet.so`
- [x] No `unsafe` outside this crate (except the allocation-counting test, decision D8)

**Decisions recorded in this task:**

- The header claimed strings were "NUL-terminated; never rely on the NUL". They are not:
  the engine hands out a pointer into its own `String`. The header now says
  length-delimited in bytes, not NUL-terminated, and that a zero length means the pointer
  must not be dereferenced.
- The panic containment test drives the real `viet_engine_feed`, not a stand-in closure: a
  `#[cfg(test)]`-only `thread_local` asks the next feed to panic, so the guard is tested
  where it lives. The injection cannot be compiled into a shipped build.
- The C smoke test found the trap the header now documents: `backspaces` counts
  characters, and `ệ` is three bytes, so a C helper that deletes bytes corrupts the text.
  `tests/smoke.c` carries `delete_characters` as the reference implementation for the
  addon.
- Both artifacts are exercised, not just built: the same C test links against `libviet.a`
  and against `libviet.so`, and the script checks that the four functions are exported
  before it links.

**Verification:**
```bash
cargo build -p viet-ffi --release
cargo test -p viet-ffi
bash scripts/c_smoke_test.sh
```

**Dependencies:** T2.3
**Files:** `crates/viet-ffi/src/lib.rs`, `crates/viet-ffi/include/viet.h`,
`crates/viet-ffi/tests/smoke.c`, `scripts/c_smoke_test.sh`
**Scope:** M

---

## T3.2 — C smoke test linked against the staticlib

**Description:** A `cc`-compiled program drives the engine through `viet.h`, so the addon can start.

**Acceptance criteria:**
- [x] Compiles with `-std=c11 -Wall -Wextra -Werror` against the header
- [x] Links against `libviet.a` and runs green, and again against `libviet.so`
- [x] Types words in Telex and VNI, checks a restore, a reusable engine, a NULL engine
- [x] CI runs it, so a header/implementation drift fails the build

### Checkpoint F — module done
- [x] Spec §9 criteria 1, 2, 5 and 6 demonstrably met (corpus, clippy, `Action` expressiveness,
      `viet-cli type "vieejt"` on a clean checkout); 3 and 4 are the fuzz run and the p99
      benchmark, still blocked on tooling approval
- [x] `docs/SPEC-engine-core.md` status set to `implemented`
- [x] `docs/PLAN.md` documents table updated
- [x] Human reviews before starting `ime-addon` (approved 2026-11-13)

---

# Phase 4 — `ime-addon`: an input method you can install

The engine is done and its C ABI is verified from C. This phase is the fcitx5 addon that
turns it into something a person types with, then the one-command install.

**Ground truth checked before planning** (2026-11-13): `pkg-config --modversion Fcitx5Core`
is `5.1.7`, headers are under `/usr/include/Fcitx5/{Core,Config,Utils}`, and the installed
CMake package exports imported targets only — there is **no** `fcitx5_addon()` helper
macro in 5.1.7, so the addon targets are declared by hand. Read the fcitx5 addon template
for the current layout before writing `CMakeLists.txt`, and do not guess the ABI names.

## T4.1 — Addon that loads

**Description:** A CMake project under `addon/` that builds `directtype.so` as an fcitx5
addon exposing one input method, whose key handler currently forwards every key
unchanged. This is the slice that proves the toolchain, the addon metadata and the
loading path before any engine call is wired in.

**Acceptance criteria:**
- [x] `cmake -S addon -B addon/build && cmake --build addon/build` produces the module
- [x] `cmake --install` places it under the prefix with its `directtype.conf` metadata
- [x] The module exports `fcitx_addon_factory_instance` and the factory builds an engine
      that lists one Vietnamese input method named `directtype`
- [x] Every key is forwarded unchanged, so turning it on cannot break typing
- [ ] Appears in `fcitx5-configtool` and loads into a running fcitx5 — **human check**,
      the daemon is not installed on this machine

**Decisions recorded in this task:**

- Three facts from the installed 5.1.7 headers, all of which the plan had guessed wrong or
  left open: there is **no** `fcitx5_addon()` CMake macro (the module and both installs are
  declared by hand), the engine base class is `InputMethodEngine`, not
  `InputMethodEngineV2`, and `Fcitx5UtilsConfig.cmake` installs to
  `${CMAKE_INSTALL_PREFIX}/lib/fcitx5` — `/usr/local/lib/fcitx5`, which fcitx5 never reads
  — unless `FCITX_INSTALL_USE_FCITX_SYS_PATHS=On`. That last one is set in
  `addon/CMakeLists.txt`, with `DIRECTTYPE_ADDON_DIR`/`DIRECTTYPE_DATA_DIR` as the
  overrides T4.6 will use for a per-user install.
- The addon is `Type=SharedLibrary`, `OnDemand=False` and lists its input method from
  `listInputMethods()`. `OnDemand=True` would mean declaring input methods in files under
  `share/fcitx5/inputmethod/`, which is the right shape only when an addon provides many.
- Verification that needs no session is automated: `addon/tests/load_check.cpp` dlopens the
  built module with `RTLD_NOW`, calls the factory, checks the instance is an
  `InputMethodEngine`, and checks the single entry's unique name, language and addon name
  (fcitx5 routes an entry back to an engine by that name, so a mismatch is a silent
  no-op). Mutation-checked: pointing the entry at another addon fails it.
- `-Wall -Wextra -Wpedantic -Werror` on our targets from the first C++ file, so the addon
  starts at the same bar as the Rust side.
- CI gained an `addon` job that installs `libfcitx5core-dev` and runs configure, build and
  ctest, so the addon cannot rot while the engine changes.

**Verification:**
```bash
cmake -S addon -B addon/build -DCMAKE_BUILD_TYPE=Release
cmake --build addon/build
ctest --test-dir addon/build --output-on-failure
```
plus a human check that fcitx5 lists the input method.

**Dependencies:** T3.2
**Files:** `addon/CMakeLists.txt`, `addon/src/addon.cpp`, `addon/src/engine.h`,
`addon/src/engine.cpp`, `addon/directtype.conf.in`, `addon/tests/load_check.cpp`,
`.github/workflows/ci.yml`
**Scope:** M

## T4.2 — Direct typing through the C ABI

**Description:** Map fcitx5 key events to `viet_input_t`, call `viet_engine_feed`, and
apply the result: delete `backspaces` **characters**, then `commitString`. Send `Boundary`
for space, enter, tab, punctuation and focus loss, and forward every key the engine does not
consume. This is the slice where the product promise becomes real: no preedit, no underline.

**Acceptance criteria:**
- [x] The addon types through the engine: `vieejt` → `việt`, `dduowngf` → `đường`,
      `asd` → `asd`, `thuowr` → `thuở`, VNI `d9u7o7ng2` → `đường`, and two words in a row
- [x] Nothing is underlined: the engine never sets a preedit string, and the addon never
      commits one
- [x] Deletions count characters, not bytes, with a forward-key fallback for applications
      that do not support surrounding text
- [x] A boundary key (space, punctuation, shortcut, focus loss) ends the word and is then
      forwarded, so the space still inserts a space
- [ ] Typing in a real application on GNOME Wayland, KDE Wayland and X11 — **human check**,
      the fcitx5 daemon is not installed on this machine

**Decisions recorded in this task:**

- The typing path lives in `src/typing.cpp` and depends on a two-method `TextTarget`
  (delete characters, insert text), not on fcitx5. `DirectTypeEngine` is the adapter that
  implements `TextTarget` over `InputContext` and classifies keys. That split is what makes
  the whole path testable without a session: `addon/tests/typing_check.cpp` types words
  against a `std::string` and checks the text, including the character-wise deletion and
  the multi-word case. Mutation-checked: dropping the deletion fails it.
- Which keystrokes are word characters is a product decision, not a Unicode property
  (`src/keys.cpp`): ASCII letters and digits, `[` and `]`, Latin-1/Extended-A/Extended-B
  (`ă ơ ư đ` on a Vietnamese layout) and Latin Extended Additional (`ệ ỹ`). Everything else
  is a boundary and reaches the application untouched. `addon/tests/key_mapping_check.cpp`
  pins it, including Ctrl/Alt shortcuts and keys that type nothing. The first version of
  that range stopped at Latin Extended-A, which made `ơ` (0x01A1) and `ư` (0x01B0) word
  *separators* — the test caught it, and a Vietnamese layout would not have typed at all.
- The engine is linked statically (`libviet.a`) so the addon ships as one module with no
  runtime path to a second library. `cargo` runs as a CMake target on every build, so the
  module can never link a stale engine; cargo itself is a no-op when nothing changed.
- Deletion uses `deleteSurroundingText` when the application advertises surrounding text and
  otherwise forwards one backspace key per character. The forward-key path is the one that
  works in terminals and older toolkits; it is untested until the human session test.

**Verification:**
```bash
cmake -S addon -B addon/build -DCMAKE_BUILD_TYPE=Release
cmake --build addon/build
ctest --test-dir addon/build --output-on-failure
```
plus the human typing test in real applications.

**Dependencies:** T4.1
**Files:** `addon/src/typing.h`, `addon/src/typing.cpp`, `addon/src/text_target.h`,
`addon/src/keys.h`, `addon/src/keys.cpp`, `addon/src/engine.h`, `addon/src/engine.cpp`,
`addon/tests/typing_check.cpp`, `addon/tests/key_mapping_check.cpp`, `addon/CMakeLists.txt`
**Scope:** M

## T4.3 — Language toggle

**Description:** Vietnamese and English as two states of one input method, with one
obvious way to switch: a toggle key handled inside the addon. The state must be visible and
must survive a restart.

**Acceptance criteria:**
- [x] Switching to English forwards every key untouched (no transformation, no restore)
- [x] The toggle key works in both languages — it is the only way back out of English
- [x] The key does not swallow anything in English mode; every other key passes through
- [x] A fresh engine is used for a new word, so a toggle never rewrites the previous word
- [x] The choice is persisted and restored on the next start
- [ ] The toggle key in a real session, and that Alt+Z does not collide with the user's
      own bindings — **human check**

**Decisions recorded in this task:**

- Vietnamese and English are two states of the addon's single input method, not two entries
  in fcitx5's input method list. The user asked for one obvious language toggle; a second
  list entry would be a second thing to configure and would forget which mode was on.
- Default toggle key is `Alt+Z`: the UniKey habit, unused by fcitx5's own bindings, and a
  key that can be pressed inside a text field. It is a config option, so T4.4 exposes it in
  `fcitx5-configtool` and it can be changed now by editing
  `~/.config/fcitx5/conf/directtype.conf`.
- The state and the key live in a `FCITX_CONFIGURATION`, persisted through fcitx5's own
  ini machinery (`readAsIni`/`safeSaveAsIni`, `StandardPath::Type::PkgConfig`) to
  `$XDG_CONFIG_HOME/fcitx5/conf/directtype.conf`. The engine reloads it in its constructor
  and saves on every toggle, so a restart — or a crash — comes back in the language the
  user left.
- Toggling drops the word in progress instead of rewriting it: the text on screen belongs to
  the application, and a language switch must never edit it. The next key starts a fresh
  word, so nothing from the previous one leaks across the switch.
- `TypingSession` (in `typing.cpp`) now owns the language flag as well as the engine, so the
  toggle behaviour is testable headlessly: `typing_check` types in English, toggles mid-word
  and checks nothing was edited, and `config_check` points `XDG_CONFIG_HOME` at a temporary
  directory, saves, constructs a second engine and checks the language came back. Both
  mutation-checked (the wrong path, and a toggle that keeps the old word).

**Verification:**
```bash
cmake -S addon -B addon/build -DCMAKE_BUILD_TYPE=Release
cmake --build addon/build
ctest --test-dir addon/build --output-on-failure
```
plus the human: type `vieejt`, press Alt+Z, type `test`, press Alt+Z again.

**Dependencies:** T4.2
**Files:** `addon/src/config.h`, `addon/src/typing.h`, `addon/src/typing.cpp`,
`addon/src/engine.h`, `addon/src/engine.cpp`, `addon/tests/typing_check.cpp`,
`addon/tests/config_check.cpp`, `addon/CMakeLists.txt`
**Scope:** M

## T4.4 — Configuration UI

**Description:** Expose the engine's five knobs through fcitx5's configuration: method
(Telex, Simple Telex, VNI), tone style (old, new), `auto_restore`, `spell_check`,
`free_tone_mark`, plus the toggle key. `Engine::set_config` requires a reset when the method
changes, and the addon must honour that.

**Acceptance criteria:**
- [x] Every field of `viet_config_t` is settable, and reaches the engine as the ABI's numbers
- [x] Changing a setting resets the engine, so a half-typed word is not finished by the new
      rule pack
- [x] Defaults match the engine's: Telex, new tone style, `auto_restore` on, `spell_check`
      off, `free_tone_mark` on
- [x] Settings survive a restart
- [ ] The options appear in `fcitx5-configtool` and the changes take effect in a session —
      **human check**, the daemon is not installed here (what can be checked headlessly is:
      the description names every enum value, so the combos are combos and not text fields)

**Decisions recorded in this task:**

- The C ABI needed a function it did not have: `viet_engine_set_config`. Adding it to
  `viet.h`, the Rust side and the C smoke test was the first half of this task, and it is
  documented to drop the word in progress — a settings change is not a typing event, and the
  text on screen was derived with the previous rule pack. The smoke test now switches a live
  engine from VNI back to Telex and back to the defaults, and the symbol is on the
  must-export list.
- The method and tone style are `FCITX_CONFIG_ENUM`s declared in namespace `fcitx`, which is
  what makes `Option`'s description dump emit `Enum/0..N`; without it `fcitx5-configtool`
  would render a text field the user cannot fill in correctly. The test checks those entries
  by name.
- The option→ABI mapping lives in `config.cpp` as `toEngineConfiguration`, not inline in the
  engine, because it is the one place where a wrong number means "VNI selected, Telex
  typed" and nothing else would notice. `config_check` asserts every enum value and every
  switch maps to its own ABI field, including the old/new tone style numbers.
- `applyConfiguration` pushes the options through `TypingSession::setEngineConfiguration`,
  which is the same seam the typing test uses, so the settings path is covered by code the
  tests already run.
- `Configurable=True` in the addon metadata: the config UI is generated from `getConfig()`,
  so there is no `.ui`/`.desktop` file to maintain.

**Verification:**
```bash
cmake -S addon -B addon/build -DCMAKE_BUILD_TYPE=Release
cmake --build addon/build
ctest --test-dir addon/build --output-on-failure
cargo test -p viet-ffi
bash scripts/c_smoke_test.sh
```
plus the human check in `fcitx5-configtool`.

**Dependencies:** T4.2
**Files:** `crates/viet-ffi/src/lib.rs`, `crates/viet-ffi/include/viet.h`,
`crates/viet-ffi/tests/smoke.c`, `scripts/c_smoke_test.sh`, `addon/src/config.h`,
`addon/src/config.cpp`, `addon/src/engine.cpp`, `addon/src/typing.h`,
`addon/src/typing.cpp`, `addon/directtype.conf.in`, `addon/tests/config_check.cpp`,
`addon/tests/typing_check.cpp`, `addon/CMakeLists.txt`
**Scope:** M

## T4.5 — Tray indicator

**Description:** The user must be able to see which language is active without opening a
settings window. Use fcitx5's own status area rather than drawing a tray icon: the addon
reports the language as a sub-mode label.

**Acceptance criteria:**
- [x] The label is the active language ("Vietnamese" / "English")
- [x] It is refreshed immediately on toggle, not on the next focus change
- [x] No second tray icon and no panel plugin: fcitx5's area shows the input method name and
      our one-word label
- [ ] Watching it change in GNOME and KDE — **human check**

**Decisions recorded in this task:**

- The seam is `subModeLabelImpl` on `InputMethodEngineV2`, and that is a real constraint:
  `InputMethodEngine::subModeLabel` dynamic-casts to V2 and returns an empty string for a
  plain engine, so overriding `subMode()` on the base shows nothing at all. Read from
  fcitx5's own `inputmethodengine.cpp`, not guessed — and it corrects an earlier note in
  this repository that V2 did not exist in 5.1.7. `load_check` now asserts the engine
  dynamic-casts to V2, which is the cheapest possible regression test for that mistake.
- The label is two words, not the method name: the status area already shows the input method
  name, so repeating it would cost space without telling the user anything. `statusLabel` is a
  pure function so the text is checked in `config_check` without an input context.
- `updateUserInterface(StatusArea, true)` after a toggle: `immediate` flushes the change, so
  the label does not lag behind the keystroke that changed it.
- No icon work: an icon would need a theme entry and a second resource to keep in sync, and
  the sub-mode text already answers "which language am I typing". If a visual distinction is
  wanted later, `subModeIconImpl` is the seam and it is one function.

**Verification:**
```bash
cmake -S addon -B addon/build -DCMAKE_BUILD_TYPE=Release
cmake --build addon/build
ctest --test-dir addon/build --output-on-failure
```
plus the human check: toggle and watch fcitx5's status area.

**Dependencies:** T4.3
**Files:** `addon/src/status.h`, `addon/src/status.cpp`, `addon/src/engine.h`,
`addon/src/engine.cpp`, `addon/tests/load_check.cpp`, `addon/tests/config_check.cpp`,
`addon/CMakeLists.txt`
**Scope:** S

## T4.6 — One-command install

**Description:** `install.sh` installs Direct Type, from a prebuilt package when one is at
hand and from source otherwise. Uninstall is one flag. It refuses to half-succeed.

**Acceptance criteria:**
- [x] `scripts/build_deb.sh` produces a Debian package with the module, its metadata and the
      documentation, and its runtime dependencies computed from the binary
- [x] `./install.sh` installs it; `--from-source` builds first; `--uninstall` removes it
- [x] Running it twice is safe (`apt-get install --reinstall`)
- [x] It fails loudly, naming the packages to install, when fcitx5 or the toolchain is missing
- [x] The module extracted **from the package** passes the addon load check
- [ ] A real install on a machine with fcitx5, then typing in an application — **human
      check**, this machine has no fcitx5 daemon

**Decisions recorded in this task:**

- The deliverable is a `.deb`, not a source-only script: the user wants to download one file
  and install it. The package is self-contained because the Rust engine is linked statically
  into the addon, and its `Depends` are computed with `dpkg-shlibdeps` from the built module
  (`libc6 (>= 2.39)`, `libfcitx5core7 (>= 5.1.7)`, …) instead of being written down by hand —
  a wrong dependency here is an addon that silently does not load. `Recommends: fcitx5`
  because the daemon is what makes the addon useful.
- Built on this machine (Ubuntu 24.04, x86_64), so the package targets Ubuntu 24.04+ on
  x86_64: `libc6 (>= 2.39)` is the hard line, and 22.04 or an ARM laptop needs a rebuild from
  source. The version is `0.0.0`, the version every manifest in the tree carries; the first
  real release version is a human decision.
- The installer installs files and restarts fcitx5, then prints the one manual step: adding
  "Direct Type" to the input method list in `fcitx5-configtool`. Editing
  `~/.config/fcitx5/profile` automatically was rejected: a malformed profile loses every
  input method the user has, and this machine has no fcitx5 to validate the result against.
  One documented click beats an untested rewrite of the user's configuration.
- `--reinstall` makes a second run a no-op rather than an error, which is what "idempotent"
  means in a script a person runs twice.
- The installer's logic was exercised with stubbed `fcitx5`, `sudo`, `apt-get` and
  `fcitx5-remote` on the PATH, so the apt invocation, the restart and the "missing
  prerequisite" branch are all checked here; only the real install needs a real machine.
- CI builds the package and runs the load check against the module *inside* it, so packaging
  cannot rot separately from the build tree.

**Verification:**
```bash
bash scripts/build_deb.sh
dpkg-deb --contents dist/directtype_*_amd64.deb
./install.sh --help
./install.sh            # on a machine with fcitx5
./install.sh --uninstall
```

**Dependencies:** T4.1–T4.5
**Files:** `install.sh`, `scripts/build_deb.sh`, `.gitignore`, `.github/workflows/ci.yml`,
`README.md`
**Scope:** M

### Checkpoint G — installable
- [ ] `./install.sh` on a clean machine, then `vieejt` types `việt` in a real app
- [ ] GNOME Wayland, KDE Wayland and X11 each verified by a human
- [ ] Uninstall leaves no trace
- [ ] Human review before publishing the repository
