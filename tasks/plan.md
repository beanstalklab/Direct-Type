# Implementation Plan: `engine-core`

Status: **awaiting human review** · Source spec: `docs/SPEC-engine-core.md` · Map: `docs/CAPABILITIES.md`

## Overview

Build the Vietnamese transformation engine as a pure Rust library, prove it correct against a golden corpus, expose it over a C ABI, and stop there. The fcitx5 addon (`ime-addon`) is a separate plan that consumes this module.

Scope of this plan: `engine-core` only. The parallel spike (`docs/SPIKE-ime-feasibility.md`) is run by the human and is not on this critical path.

## Architecture Decisions

| # | Decision | Rationale |
|---|---|---|
| A1 | Engine returns an `Action`; it never performs I/O | Testable without fcitx5; one core serves Preedit, SurroundingText and ForwardKey strategies unchanged |
| A2 | Raw keystroke sequence is retained alongside the transformed text | Backspace and auto-restore both require re-deriving from raw input |
| A3 | Zero runtime crates; static table for precomposed Vietnamese | Hot path is a per-keystroke call; avoids audit surface and build weight (pending human confirmation, T0.1) |
| A4 | Corpus lives at workspace root, not inside the crate | The addon's integration tests reuse it |
| A5 | `viet-cli check` exits `0/1/2` | Makes the corpus a CI gate before any UI exists |
| A6 | `viet-core` is split by method (`telex.rs`, `vni.rs`) over a shared syllable model | Methods differ only in key mapping; tone logic must not be duplicated |

## Task List

### Phase 0 — Decisions and skeleton

- [ ] **T0.1 Record blocking decisions**
- [x] **T0.2 Bootstrap toolchain** (requires `sudo`)
- [x] **T0.3 Scaffold Cargo workspace**
- [x] **T0.4 CI: fmt + clippy + test**

**Checkpoint A:** `cargo test --workspace` runs (0 tests), `clippy -D warnings` clean, CI green.

### Phase 1 — Correctness, vertical slice by slice

Each task ends with something you can type into and observe via `viet-cli`.

- [x] **T1.1 Buffer, boundary, passthrough + corpus runner**
- [x] **T1.2 Telex letter modifiers**
- [x] **T1.3 Syllable parser + tone index**

**Checkpoint B:** `viet type --method telex "vieejt"` prints `việt`; parser unit tests cover `qu`, `gi`, 1/2/3-vowel nuclei, coda/no-coda.

- [x] **T1.4 Telex tone keys, tone removal, free tone mark**
- [x] **T1.5 Auto-restore and spell-check**
- [x] **T1.6 Case handling (lower/UPPER/Title/mixed)**

**Checkpoint C:** `telex-new.tsv` and the old-style files (`telex-tone-old.tsv`, `telex-case-old.tsv`) green; every rule in spec §3.4 has 3 cases plus a counter-example; case invariance and precomposed output hold over the whole corpus. The 2,000-case target still waits on Q2.

- [x] **T1.7 VNI method**
- [x] **T1.8 SimpleTelex method**

**Checkpoint D:** every corpus file green (`telex-*`, `vni-*`, `simple-telex-*`, 630 cases); ≥ 2,000 cases still waits on Q2; no method-specific logic leaked into `tone.rs`. Phase 1 is complete apart from that target.

### Phase 2 — Robustness

- [x] **T2.1 Backspace and mid-composition editing**
- [ ] **T2.2 Property tests + fuzz target** — invariants, the word cap and the Unicode
      table check are done; the fuzz target waits on `cargo-fuzz` approval
- [ ] **T2.3 Benchmarks and allocation check** — the counting-allocator check is done; the
      p99 benchmark waits on `criterion` approval

**Checkpoint E:** fuzz 10⁶ inputs clean; p99 `feed` < 1 ms; zero allocations after warmup.

### Phase 3 — Boundary for the addon

- [ ] **T3.1 `viet-ffi`: C ABI, header, panics contained**
- [ ] **T3.2 C smoke test linked against the staticlib**

**Checkpoint F:** `cc`-compiled test program drives the engine through `viet.h`; `ime-addon` can start.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Tone-placement edge cases are a long tail; spec §3.4 may be incomplete | High — this is the product | Corpus-first: no rule ships without cases; corpus is reviewable text, not code |
| Corpus source is not defined (2,000 cases do not appear by themselves) | High — blocks Checkpoint C | T0.1 decides the source; see Open Question Q2. Do not start T1.2 before it is answered |
| Hand-written rules diverge from UniKey behaviour users expect | Medium | Keep `auto_restore` and `spell_check` as switches, so disagreement is a setting, not a bug report |
| Zero-dep Unicode table is hand-generated and could be wrong | Medium | Generate it from a cited Unicode data version; test all 134 mappings round-trip |
| Scope creep into macros/conversion/UI | Medium | Those are explicitly outside the capability map; reject in review |
| `free_tone_mark` interacts with auto-restore in surprising ways | Low | Closed in T1.5: `auto_restore` acts at the boundary only, `spell_check` while typing, and a property test compares a restoring engine against a non-restoring one over 2,000 generated words |

## Open Questions

- **Q1 (blocks T0.3):** confirm zero runtime crates + static table, vs `unicode-normalization`.
- **Q2 (answered for T1.2, T1.4 and T1.5):** corpus source. All three tasks used
  hand-written cases derived from spec §3.2/§3.4/§3.5 — no imported word list, no
  third-party oracle; 152 tone cases, 22 old-style tone cases and 17 restore cases now
  pass. T1.5 did consult `fcitx5-bamboo`'s core for the *closed spelling inventories*
  (onset/nucleus/coda, `spelling.go`, MIT), which is a source for a table, not an
  oracle for behaviour. Still open: how the corpus grows past hand-written cases to
  Checkpoint C's 2,000. Options: import a public Vietnamese word list (needs a licence
  check) or compare against `fcitx5-bamboo` as an oracle (best coverage, needs its own
  harness).
- **Q3 (answered):** default `tone_style`: `New` (`hòa/thủy`). Implemented as
  `ToneStyle::default()` in T1.3 and exercised by `telex-tone.tsv` in T1.4.
- **Q4 (partly answered):** project name + licence: `directtype`, `MIT OR Apache-2.0`
  (spec §10 D2/D5). Still human work: the working directory is named `unikey` and must
  be renamed before publication, and the copyright holder is recorded as `beanstalklab`
  and needs confirming before the first release.
- **Q5 (answered):** `free_tone_mark` in v1, default on (spec §10 D4), implemented in
  T1.4 and pinned by the T1.5 corpus case `test → tét`.

## Out of Scope

`ime-addon`, `install`, `compat`, settings GUI, macros, encoding conversion, non-Vietnamese languages.
