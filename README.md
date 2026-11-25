# Direct-Type

A Vietnamese input method for Linux, built on a pure transformation engine that knows
Vietnamese orthography and nothing else.

The engine is **direct**: it answers a keystroke with "delete these characters, insert
these", so a frontend *can* type without a composition buffer and without the underline
that composition-based input methods draw. The fcitx5 addon that ships today uses that path
wherever the application allows it — Chromium, most GTK/Qt toolkits — and keeps the word in
a preedit only where rewriting is impossible, because Linux applications cannot be relied on
to let an input method rewrite text they have already received: Sublime Text accepts the
commit and drops the deletion, and draws no preedit either. UniKey and Bamboo keep a preedit
everywhere for the same reason. The strategy is decided once per word, never mid-word, since
changing it halfway would duplicate or delete text.

The first language is Vietnamese. English needs no transformation, so it works as the
identity path. The engine is a rule pack, not a Vietnamese-specific program.

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
- 650 corpus cases and CI that runs formatting, lint, tests and the corpus.

The fcitx5 addon types through the engine, has its language toggle and its settings: one
input method named "Direct Type", the word shown in a preedit and committed at the word
boundary, Alt+Z to switch between Vietnamese and English, and method, tone placement,
`auto_restore`, `spell_check`, `free_tone_mark` and the toggle key in
`fcitx5-configtool` — all remembered across restarts in
`~/.config/fcitx5/conf/directtype.conf`. fcitx5's status area shows the active language next
to the input method name.

The word reaches the application through a **preedit** by default: it is drawn in the text
field, underlined, and committed at the word boundary. That is the floor Linux offers,
because the alternative — commit every keystroke and delete what changed — depends on the
application honouring `deleteSurroundingText`, and several do not: Sublime Text and Chrome's
address bar accept the commit and drop the deletion, which turns `Đỗ` into `DĐoôỗ`. Nothing
in the protocol lets an input method tell those fields from the ones that work, and Chrome
disagrees with itself (its page content honours the deletion, its address bar does not), so
the rewriting modes are a deliberate choice rather than a guess.

`Delivery=SurroundingText` (or `Backspace`) switches an application you have seen honour the
deletion to per-keystroke rewriting, without the underline. To see what a field offers, run
`DIRECTTYPE_DEBUG=1 fcitx5 --replace` in a terminal: one line per word reports the program,
the frontend, whether surrounding text was usable, and the delivery in use.

## Install

```console
$ bash scripts/build_deb.sh          # or download the .deb
$ ./install.sh                       # needs sudo; --uninstall removes it
```

Then add "Direct Type" to the input method list in `fcitx5-configtool` and press Apply.
The package targets **Ubuntu 24.04+ on x86_64** (`libc6 (>= 2.39)`, `libfcitx5core7`);
on another distribution or architecture, `./install.sh --from-source` builds it there.
It needs fcitx5 itself, which the installer checks for and names if it is missing.

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

## Why the engine is "direct"

A composition-based IME builds a preedit string that the application renders underlined
and only commits at a word boundary. Direct typing replaces that model: the engine knows
what the word should look like at every keystroke, so the frontend can commit the result
immediately and rewrite what is already on screen when a modifier changes it.

On Linux that last step is the hard part. `deleteSurroundingText` is optional for
applications and several of them accept the commit while ignoring the deletion, so the
addon asks the application whether it can be rewritten and uses the direct path only when
the answer is trustworthy; otherwise it keeps the word in a preedit, like the other
Vietnamese input methods for fcitx5. See `docs/PLAN.md` for the strategy table and the open
per-application question.

## Licence

Licensed under either of

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT licence ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
