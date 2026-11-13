# Direct-Type

A multi-language input method for Linux, built around **direct typing**: keystrokes
reach the application immediately, without a composition buffer, and therefore
without the underline that composition-based input methods draw.

The first language is Vietnamese. English needs no transformation, so it works as
the identity path and exercises the language toggle. The engine is a rule pack, not
a Vietnamese-specific program.

## Status

Pre-alpha. What exists today:

- `crates/viet-core` — the pure transformation engine. Characters commit
  immediately; the raw word is tracked and re-derived on every key, so a Telex
  modifier rewrites what the application already shows (`aa` → `â`) without a
  composition buffer. Telex letters and tone keys work, including old/new tone
  placement and `free_tone_mark`; `auto_restore` and `spell_check` work on a closed
  onset/nucleus/coda spelling test; case is per letter, so `VIEEJT`, `Vieejt` and
  `vIeejt` come out as `VIỆT`, `Việt` and `vIệt`. Three methods work from one shared
  fold: Telex, VNI (digits `1`–`5`, `0`, `6`–`9`) and Simple Telex (`aa/ee/oo`, `w`,
  `dd` and the Telex tones).
- `crates/viet-cli` — `viet-cli type` and `viet-cli check` (golden-corpus runner,
  exit codes `0`/`1`/`2`).
- `crates/viet-ffi` — a C ABI over the engine, with panics contained at the boundary;
  `crates/viet-ffi/tests/smoke.c` is a C caller linked against both library artifacts.
- 630 corpus cases and CI that runs formatting, lint, tests and the corpus.

The fcitx5 addon types through the engine (T4.2), has its language toggle (T4.3) and its
settings (T4.4): one input method named "Direct Type", text committed straight into the
application with no preedit, Alt+Z to switch between Vietnamese and English, and method,
tone placement, `auto_restore`, `spell_check`, `free_tone_mark` and the toggle key in
`fcitx5-configtool` — all remembered across restarts in
`~/.config/fcitx5/conf/directtype.conf`. It has no status indicator and no installer yet
(T4.5, T4.6), so there is still nothing to install.

## Build and test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p viet-cli -- check tests/corpus
python3 scripts/verify_unicode_table.py    # the table against Unicode
bash scripts/c_smoke_test.sh               # the C ABI, from C

cmake -S addon -B addon/build -DCMAKE_BUILD_TYPE=Release
cmake --build addon/build
ctest --test-dir addon/build --output-on-failure   # the addon loads
```

Type through the engine without any GUI:

```console
$ cargo run -p viet-cli -- type --method telex "dduowng"
đương
```

Tone keys work too, so the whole word comes out at once:

```console
$ cargo run -p viet-cli -- type --method telex "vieejt"
việt
```

Auto-restore works on the word boundary, so a word the engine cannot make Vietnamese
comes back as the keystrokes that typed it:

```console
$ cargo run -p viet-cli -- type --method telex "asd"
asd
```

A word that is Vietnamese keeps its marks, and a word that only *happens* to be
Vietnamese keeps them too — `test` becomes `tét`, because `tét` is a real syllable
(`nét`, `mét`, `sét` share its rhyme). Typing English needs the language toggle, or
`free_tone_mark` off, which leaves the `s` literal.

## Documentation

- [`docs/PLAN.md`](docs/PLAN.md) — goals, architecture, milestones, risks
- [`docs/CAPABILITIES.md`](docs/CAPABILITIES.md) — module map and build order
- [`docs/SPEC-engine-core.md`](docs/SPEC-engine-core.md) — engine spec and decisions
- [`docs/SPIKE-ime-feasibility.md`](docs/SPIKE-ime-feasibility.md) — platform feasibility spike

## Why "direct"

Composition-based IMEs build a preedit string that the application renders
underlined and only commit at a word boundary. That model is why typing Vietnamese
on Linux feels like it "needs a spacebar", and why some applications show nothing
until then. Direct typing removes the composition state instead of restyling it.

## Licence

Licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT licence ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
