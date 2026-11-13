# AGENTS.md

Project rules for agents working in this repository.

## Status: engine-core in progress

The greenfield Rust engine is being built task by task from `tasks/plan.md`;
`docs/PLAN.md` remains the source of truth for goals, architecture, milestones and
open questions. Read both before proposing code.

The Python scaffold from the original project template has been removed. Do not
infer the product architecture from anything the template left behind.

Stack is still a draft. Treat every stack statement in `docs/PLAN.md` as
"current decision, may change", and confirm before depending on it. The fcitx5
addon, the installer and the C++ build do not exist yet.

## Core rules

- Do not write implementation code before a spec or plan entry exists in `docs/PLAN.md`.
- If a task matches a skill, invoke it with the `skill` tool before acting.
- Follow the loaded skill's workflow fully; do not skip its required steps.
- When a plan decision changes, update `docs/PLAN.md` in the same change.
- Do not add runtime dependencies, build tooling or lockfiles without asking first.
- Do not present placeholder or template code as the product architecture.

## Naming, comments and documentation

Applies to variable names, function names, comments, doc comments and docs.

- **No abbreviations.** Do not abbreviate names. `userConfig`, not `usrCfg`;
  `bufferLength`, not `bufLen`; `initialize`, not `init`. Widely accepted domain
  terms (`utf8`, `api`, `css`, `ime`, `keysym`) are fine when they are the
  clearest name available; spell out everything else.
- **Optimize for being understood, not for being short.** A reader should picture
  what a name refers to without opening another file. Prefer `remainingKeystrokes`
  over `n`, `restoreOriginalWord` over `fix`.
- **Use common vocabulary.** Everyday words in the language of the codebase, not
  invented jargon, not synonyms of synonyms. If two names mean the same thing, use
  one of them everywhere.
- **Comments explain why, not what.** State the reason, the constraint, or the
  trade-off. Do not restate the code. Keep comments and docs current in the same
  change as the code they describe; a stale comment is a defect.

## Design

Follow SOLID:

- **Single responsibility**: one reason to change per type, function and module.
  Split anything that mixes policy, I/O and formatting.
- **Open/closed**: extend behavior through new implementations of an existing
  seam, not by editing every call site.
- **Liskov substitution**: an implementation may only strengthen guarantees,
  never weaken them. Document preconditions and postconditions.
- **Interface segregation**: small, purpose-built interfaces; no caller should
  depend on a method it does not use.
- **Dependency inversion**: the hot path depends on abstractions it owns, not on
  fcitx5, the OS or the filesystem. Keep pure logic free of I/O so it stays
  testable without a session.

When SOLID conflicts with clarity, prefer clarity and say why in the change.

## Intent -> skill mapping

- Vague idea / unclear requirements -> `interview-me`, then `idea-refine`
- New feature or design decision -> `spec-driven-development`
- Turning a spec into work -> `planning-and-task-breakdown`
- Implementing a task -> `incremental-implementation` + `test-driven-development`
- Anything spiking unproven platform behavior -> `doubt-driven-development`
- Bug, regression, or "it worked yesterday" -> `debugging-and-error-recovery`
- Tricky hot path or latency concern -> `performance-optimization`
- Using a framework/API where correctness matters -> `source-driven-development`
- Reviewing a change -> `code-review-and-quality`, then `code-simplification`
- Recording a decision -> `documentation-and-adrs`
- Commits, branches, releases -> `git-workflow-and-versioning`

## Verification

Current phase: **Phase 4 — `ime-addon`, T4.4 done (typing + toggle + settings), T4.5 (status) next**. The commands below are the only accepted
verification; run them from the repository root. Do not substitute different
commands, and do not lower a check to get to green.

### Current state (2026-11-13)

- **Toolchain**: rustc/cargo 1.99.1 + clippy + rustfmt (user-local, `~/.cargo`),
  cc 13.3.0, pkg-config 1.8.1, cmake 3.28.3, `Fcitx5Core` dev headers 5.1.7.
  rustup was installed with `--no-modify-path`: run `source "$HOME/.cargo/env"`
  before any cargo command (or add it to your shell profile).
- **T1.1 done**: characters commit immediately (no preedit), `viet-cli type` and
  `viet-cli check` work with exit codes `0/1/2`, and CI runs the corpus check.
- **T1.2 done**: Telex letter modifiers (`aa/ee/oo`, `w`, `aw/ow/uw`, `dd`, `[`, `]`,
  `uowng → ương`, `uow → uơ`) with doubling to undo; a modifier reports `backspaces` and rewrites
  text the application already shows.
- **T1.3 done**: `syllable::parse` (onset/nucleus/coda, `qu` and `gi` as onsets,
  fixed-array so it never allocates) and `tone::tone_index` (quality diacritic, coda,
  then open-syllable rules; `oa/oe/uy` are the only style-dependent pairs).
- **T1.4 done**: Telex tone keys `s f r x j`, `z` and tone-key doubling remove a tone,
  `free_tone_mark` on (default) and off, old/new placement. The mark is placed from
  the finished syllable, so `toasn → toán`; a tone key outside a syllable stays a
  letter (`sz → sz`) and a word that never becomes one syllable keeps every keystroke
  (`ashello`). Checkpoint B (`vieejt` → `việt`) closed.
- **T1.5 done**: `auto_restore` (default on) gives the raw keystrokes back at the word
  boundary when the finished word is not a Vietnamese syllable (`asd`, `asdf`, `oow`),
  and `spell_check` (default off) refuses a transformation the word can no longer
  justify while typing (`asd` never shows `ád`, `aaa` stays `aaa`). Spelling validity
  is a closed-set onset/nucleus/coda test in `crates/viet-core/src/spelling.rs`, whose
  inventories come from `fcitx5-bamboo`'s `spelling.go`; it over-generates on purpose,
  because a wrongly rejected word loses the user's diacritics. 247 corpus cases
  (152 tone + 22 old-style + 17 restore), 68 core tests, including a property test
  that a restore never loses or reorders a keystroke.
- **T1.6 done**: case is per letter (§3.7). A letter keeps the case of the key that
  produced it, a modifier or tone key never sets a vowel's case (`vIeejt → vIệt`), an
  undo keeps the case already on screen (`Ww → W`, `AaA → AA`), and shift is not a
  different key (`ASs → AS`). Two bugs came out of the new corpus-level property:
  tone-key doubling compared characters case-sensitively, and the `w` undo replaced the
  inserted `ư` with the current key, flipping the capital. 353 corpus cases (106 new
  case cases across `telex-case.tsv` and `telex-case-old.tsv`), 71 core tests, 4 CLI
  integration tests, including case invariance for every corpus case and the
  precomposed-output (NFC) check.
- **T1.7 done**: VNI works from its own rule pack (`src/vni.rs`) on the shared fold
  (`src/input.rs`, `RulePack` trait): `1..5` tones, `0` removes a tone, `6/7/8/9` add
  the circumflex, horn, breve and stroke, `uo7` horns both letters while the word goes
  on. `0` drops the tone digit (`a10 → a`) while doubling keeps one (`a11 → a1`). Keys
  do not leak between methods (`w`, `[`, `]`, `s`, `z` are letters in VNI). 579 corpus
  cases (204 VNI new + 17 VNI old), 80 core tests, 4 CLI integration tests, and
  `tone.rs` is unchanged since Checkpoint C.
- While writing the VNI `7` rule, T1.2's eager `uow → ươ` turned out to make `thuở`,
  `quơ` and `huơ` untypable. Fixed in its own commit (`f89b2f2`): a word that ends at
  the horn key keeps `uơ`, a word that goes on horns the pair (`uowng → ương`).
- **T1.8 done**: Simple Telex is a third `RulePack` in `telex.rs` — Telex's tone keys
  and placement, with only `aa/ee/oo`, `w` and `dd` as letter modifiers. `w` produces
  `ư` only while the word is an onset, so `aw` stays `aw` with `auto_restore` off as
  well. 52 corpus cases in `simple-telex-new.tsv`, 630 in total; 81 core tests.
- All three methods (Telex, VNI, Simple Telex) now share one fold in `src/input.rs`;
  `tone.rs` has not changed since Checkpoint C.
- **T2.1 done**: backspace needed no engine change — it already deleted a raw key and
  re-derived the word — so the task added the guarantee: every prefix of every corpus
  case, deleted and finished, must equal the shorter prefix typed and finished (about
  two thousand prefixes per run); a random property that a word with deletes ends where
  its surviving keys would; and unit tests for the forwarded backspace after a normal
  and after a restored boundary, for a delete that brings a tone back (`asz → as` →
  `á`), and for `spell_check` re-applying to the shorter word. 88 core tests, 5 CLI
  integration tests.
- **T2.2 partly done**: `crates/viet-core/tests/invariants.rs` holds the properties the
  public API can state — every configuration (3 methods x 2 tone styles x 3 switches),
  hostile input (controls, combining marks, emoji, U+10FFFF), `reset` totality, a
  forwarded boundary, `preedit` always empty, and no delete reaching past the word. No
  `proptest`: the generator is a seeded LCG per test file, so the zero-dependency promise
  holds and a failure is reproducible. Each property was checked against a deliberate
  mutation and failed.
- The raw-key buffer is capped at 64 keys: the word ends where it stands and the next key
  starts a new one, in one action with no deletions. 89 core tests, 5 invariant tests,
  5 CLI tests.
- `scripts/verify_unicode_table.py` now exists and CI runs it: 144 table entries checked
  against Unicode canonical decomposition. `unicode.rs` claimed this script since T1.1
  and the claim was false — fixed, not restated.
- Not done, blocked on approval: the `cargo-fuzz` target (`libfuzzer-sys`, nightly) and
  the `criterion` benchmark. The user chose dependency-free work first, so Checkpoint E's
  fuzz run stays open.
- **T2.3 partly done**: `crates/viet-core/tests/allocations.rs` proves `feed` allocates
  nothing after warmup, for all three methods, the corpus' hardest words and a word long
  enough to cross the 64-key cap; zero allocations in debug and release. This is the only
  `unsafe` outside `viet-ffi` (spec D8, §8 amended). Mutation-checked: a one-line
  allocation per keystroke fails it with 14,100 allocations.
- Still blocked on approval: the `cargo-fuzz` target and the `criterion` p99 benchmark.
- **T3.1 and T3.2 done**: the C ABI is verified from C, not only from Rust.
  `crates/viet-ffi/tests/smoke.c` is compiled with `-std=c11 -Wall -Wextra -Werror` and
  linked against `libviet.a` and `libviet.so` by `scripts/c_smoke_test.sh`, which also
  checks that the four functions are exported; CI runs it. The header's false NUL claim is
  fixed (spans are length-delimited in bytes), and a `#[cfg(test)]`-only injection proves a
  panic inside the feed is contained. The C test also found that `backspaces` counts
  characters, not bytes — `delete_characters` in `smoke.c` is the reference for the addon.
  Checkpoint F is met.
- **T4.1 done**: `addon/` builds `directtype.so` with CMake, installs it plus
  `directtype.conf` into fcitx5's addon and data directories, and exports
  `fcitx_addon_factory_instance`. `addon/tests/load_check.cpp` is a CTest that dlopens the
  built module, calls the factory and asserts the engine lists one Vietnamese input method
  named `directtype`; CI builds and runs it. Every key is still forwarded unchanged, so
  enabling it cannot break typing.
- Facts pinned while building it, because guessing cost time in this phase: fcitx5 5.1.7's
  CMake package has **no** `fcitx5_addon()` helper (declare the module and installs by
  hand), the engine base class is `InputMethodEngine` (**not** `InputMethodEngineV2`, which
  is not in 5.1.7), and `Fcitx5UtilsConfig.cmake` defaults the install dirs to
  `${CMAKE_INSTALL_PREFIX}/lib/fcitx5` (`/usr/local/...`, a directory fcitx5 never reads)
  unless `FCITX_INSTALL_USE_FCITX_SYS_PATHS=On`. `DIRECTTYPE_ADDON_DIR` and
  `DIRECTTYPE_DATA_DIR` are the cache variables to override for a per-user install.
- **T4.2 done**: the addon types through the C ABI. The typing path lives in
  `addon/src/typing.cpp` and depends on a two-method `TextTarget` (delete characters, insert
  text) instead of fcitx5, so `addon/tests/typing_check.cpp` types real words headlessly —
  `vieejt` → `việt`, `dduowngf` → `đường`, `asd` → `asd`, `thuowr` → `thuở`, VNI, two words
  in a row. `src/keys.cpp` decides which keys are word characters (ASCII letters and digits,
  `[`, `]`, Latin-1/Extended-A/Extended-B, Latin Extended Additional), and everything else
  ends the word and is forwarded. Deletion uses `deleteSurroundingText` when the app
  supports surrounding text, otherwise forwarded backspaces. Three addon ctest cases, all
  mutation-checked.
- **T4.3 done**: Vietnamese and English are two states of one input method. `Alt+Z` toggles,
  the flag and the key live in a `FCITX_CONFIGURATION` persisted to
  `$XDG_CONFIG_HOME/fcitx5/conf/directtype.conf` and reloaded in the engine constructor, and
  toggling drops the word in progress instead of rewriting it. `TypingSession` owns the flag,
  so `typing_check` covers English passthrough and mid-word toggling headlessly, and
  `config_check` points `XDG_CONFIG_HOME` at a temporary directory and proves the language
  comes back on the next engine. Four addon ctest cases, all mutation-checked.
- **T4.4 done**: the five engine knobs and the toggle key are fcitx5 config options
  (`Configurable=True`, so the UI is generated), mapped to `viet_config_t` in one function
  that the tests check value by value. The C ABI gained `viet_engine_set_config`, which drops
  the word in progress; the smoke test switches a live engine between methods, and the symbol
  is on the must-export list. Settings persist to
  `~/.config/fcitx5/conf/directtype.conf` and are asserted after a restart.
- **The fcitx5 daemon is not installed on this machine** (only `libfcitx5core-dev` and the
  frontends), so loading into a running session, `fcitx5-configtool`, the status indicator and
  typing in real applications are human checks on a machine that has fcitx5.
- Nothing is installable yet: no status indicator, no installer. Next is T4.5.
- `test` → `tét` stays `tét`: `tét` is real Vietnamese (`nét`, `mét`, `sét` share its
  rhyme), so no spelling rule may rewrite it. English typing needs the toggle key, or
  `free_tone_mark` off. Do not "fix" this by weakening the tables; the corpus case
  `telex-restore.tsv test → tét` exists to stop that.
- No input method is missing any more; what remains before the addon is robustness
  (T2.x: backspace, property tests, benchmarks) and the corpus target of Q2.
- C ABI names follow the AGENTS.md naming rules (no abbreviations): `code_point`,
  `commit_length`, `preedit_length`, `reserved_flags`, `configuration`, `engine`.
- Product name: **`directtype`**; the public repository is
  `beanstalklab/Direct-Type`. The working directory is still named `unikey` and must
  be renamed before anything is published under that path.


```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cmake -S addon -B addon/build -DCMAKE_BUILD_TYPE=Release
cmake --build addon/build
ctest --test-dir addon/build --output-on-failure
cargo run -p viet-cli -- check tests/corpus     # exit 0/1/2
cargo bench -p viet-core                        # from T2.3 onwards
```

CI (`.github/workflows/ci.yml`) runs formatting, lint, tests, build, the corpus check, the
Unicode table check and the C ABI smoke test on every push and PR, plus one `addon` job
that installs `libfcitx5core-dev` and builds and ctest's the addon. The benchmark is not
in CI yet (T2.3 is blocked on `criterion`).

Undefined until the relevant task exists: the installer test (T4.6). Ask before
inventing it. The addon build is defined: `addon/CMakeLists.txt`, CMake 3.16+, fcitx5
5.1.7 headers, no `fcitx5_addon()` macro.
