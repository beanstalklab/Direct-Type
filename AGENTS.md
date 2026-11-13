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

Current phase: **T1.5 (auto-restore and spell-check)**. The commands below are the only accepted
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
  `uow → ươ`) with doubling to undo; a modifier reports `backspaces` and rewrites
  text the application already shows.
- **T1.3 done**: `syllable::parse` (onset/nucleus/coda, `qu` and `gi` as onsets,
  fixed-array so it never allocates) and `tone::tone_index` (quality diacritic, coda,
  then open-syllable rules; `oa/oe/uy` are the only style-dependent pairs).
- **T1.4 done**: Telex tone keys `s f r x j`, `z` and tone-key doubling remove a tone,
  `free_tone_mark` on (default) and off, old/new placement. The mark is placed from
  the finished syllable, so `toasn → toán`; a tone key outside a syllable stays a
  letter (`sz → sz`) and a word that never becomes one syllable keeps every keystroke
  (`ashello`). 230 corpus cases (152 tone cases + 22 old-style tone cases), 51 core
  tests. Checkpoint B (`vieejt` → `việt`) closed.
- Next: T1.5 — `auto_restore` and `spell_check`. Today an invalid word commits the
  letter transformation with every tone key literal; a Vietnamese buffer is already
  correct, so T1.5 is about `test` → `tét` becoming `test` again.
- Not implemented, by design: auto-restore and spell-check (T1.5), case matrix (T1.6),
  VNI (T1.7), Simple Telex's restricted rule set (T1.8). Those methods currently pass
  letters through unchanged.
- C ABI names follow the AGENTS.md naming rules (no abbreviations): `code_point`,
  `commit_length`, `preedit_length`, `reserved_flags`, `configuration`, `engine`.
- Product name: **`directtype`**; the public repository is
  `beanstalklab/Direct-Type`. The working directory is still named `unikey` and must
  be renamed before anything is published under that path.


```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus     # exit 0/1/2
cargo bench -p viet-core                        # from T2.3 onwards
```

CI (`.github/workflows/ci.yml`) runs formatting, lint, tests, build and the corpus
check on every push and PR. The benchmark joins CI at T2.3.

Undefined until the relevant task exists: the C++ addon build (needs CMake + fcitx5
headers, T0.2) and any installer test. Ask before inventing either.
