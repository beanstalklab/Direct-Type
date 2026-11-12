# AGENTS.md

Project rules for agents working in this repository.

## Status: planning

This project has **no agreed implementation yet**. `docs/PLAN.md` is the source of
truth for goals, architecture, milestones and open questions. Read it before
proposing code.

The current source tree (`src/unikey/`, `pyproject.toml`) is scaffolding from a
project template, **not** a description of the target stack. Do not infer
architecture, package layout or runtime language from it.

Stack is still a draft. Treat every stack statement in `docs/PLAN.md` as
"current decision, may change", and confirm before depending on it.

## Core rules

- Do not write implementation code before a spec or plan entry exists in `docs/PLAN.md`.
- If a task matches a skill, invoke it with the `skill` tool before acting.
- Follow the loaded skill's workflow fully; do not skip its required steps.
- When a plan decision changes, update `docs/PLAN.md` in the same change.
- Do not add runtime dependencies, build tooling or lockfiles without asking first.
- Do not present the placeholder scaffold as the product architecture.

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

Current phase: **T0.2 (toolchain bootstrap)**. These commands are defined by
`docs/SPEC-engine-core.md` §4 and `tasks/todo.md`, but **are not runnable until
T0.2 and T0.3 complete** (this host has no Rust toolchain yet). Do not substitute
different commands; if one fails because the toolchain is absent, report that
instead of improvising.

### Current state (2026-11-12)

- **Toolchain complete**: rustc/cargo 1.99.1 + clippy + rustfmt (user-local, `~/.cargo`),
  cc 13.3.0, pkg-config 1.8.1, cmake 3.28.3, `Fcitx5Core` dev headers 5.1.7.
  rustup was installed with `--no-modify-path`: run `source "$HOME/.cargo/env"`
  before any cargo command (or add it to your shell profile).
- **T0.3 verified**: `cargo build/test/clippy/fmt` all pass; `viet-cli` exits `2`
  on usage as specified; FFI artifacts build as `target/debug/libviet.a` and
  `libviet.so` (crate `viet-ffi`, `[lib] name = "viet"`).
- **T1.1 done**: characters commit immediately (no preedit), `viet-cli type` and
  `viet-cli check` work with exit codes `0/1/2`, 10 passthrough corpus cases pass,
  and CI runs the corpus check. 7 core unit tests + 2 CLI tests green.
- Next: T1.2 — blocked on the corpus source decision (`tasks/plan.md` Q2).
- C ABI names follow the AGENTS.md naming rules (no abbreviations): `code_point`,
  `commit_length`, `preedit_length`, `reserved_flags`, `configuration`, `engine`.
- Product name: `directtype` proposed (language-neutral), awaiting confirmation before any public artifact.


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
