# Vietnamese IME for Ubuntu — Project Plan

Status: draft v0.1 (decisions from 2026-11-12)
Working name: `unikey` (placeholder — must be renamed before release, the name is a trademark of the original Windows product)

## 0. Documents

| File | Status | Purpose |
|---|---|---|
| `docs/PLAN.md` | this file | Goals, architecture, milestones, risks |
| `docs/CAPABILITIES.md` | approved | Module map and build order; gate for all module specs |
| `docs/SPIKE-ime-feasibility.md` | ready to run (needs GUI + sudo) | Phase 0 investigation: exact commands, app matrix, go/no-go on key injection |
| `docs/SPEC-engine-core.md` | approved; Q2 corpus size open | Full spec for the first implementation module |
| `tasks/plan.md` | approved | Implementation order, risks, decisions for `engine-core` |
| `tasks/todo.md` | T1.1–T1.4 done; T1.5 next | Task-by-task acceptance criteria and verification commands |

Per `AGENTS.md`, no implementation code is written before `CAPABILITIES.md` and the module spec are approved; both are, and `tasks/plan.md` drives the work. When a plan decision changes, this file changes in the same commit.

## 1. Goal

A Vietnamese input method for Ubuntu that behaves like UniKey on Windows:

- Typing produces final characters immediately. No composition underline, no "press space to finish the word".
- One obvious shortcut toggles Vietnamese/English. No layout puzzles.
- Installable with a single command.
- Works in every app the user actually types in, including ones with poor IME support.
- Engine designed for later languages without redesign.

Non-goals for v1: encoding conversion (Unicode/TCVN3/VNI-Windows), VIQR, auto-correct dictionary, cloud sync, mobile.

## 2. Why the current tools fail

Two different IME architectures are being conflated:

| | Composition IME (IBus, fcitx5 engines) | Keystroke rewriter (UniKey/Windows) |
|---|---|---|
| Mechanism | engine builds a *preedit* string; app renders it | hook swallows key, emits final char |
| Underline | drawn by the app for preedit (by design) | none — nothing is in composition |
| Apps without preedit rendering (Sublime, some Electron) | text invisible until commit | works normally |

**Decision 2026-11-13 (supersedes the "no composition underscore" goal for v1):** the v1
addon uses the **preedit** strategy for every application. The direct path needs
`deleteSurroundingText`, and Sublime Text and some terminals accept the commit while
dropping the deletion, which produces visible garbage (`Đỗ` → `DĐoôỗ`). UniKey and Bamboo
hold a preedit for the same reason. The engine stays direct-capable — it answers with
delete-and-insert, which a frontend that can rewrite text uses directly — so the preedit is
a frontend choice, revisitable per application once the survival matrix from the spike
exists. Consequence: the name `Direct-Type` now describes the engine, not what the addon
does today; the product name decision (D5) should be revisited before publishing.
| Word finalization | needs an explicit commit (space/punctuation) | none |

`Super+Space` is a third mechanism: GNOME's XKB *layout* switcher (`us` ↔ `vn`), unrelated to IME mode. Using it as an IME toggle cannot work while GNOME owns the binding. The installer must take ownership of the shortcut explicitly.

## 3. Decisions

| Topic | Decision | Rationale |
|---|---|---|
| Engine | Greenfield | UX/strategy layer is the differentiator; Rust core wanted; avoids GPL fork and upstream drag |
| Hot path language | Rust core, C ABI, thin C++ addon | Safe, fast, testable standalone; C++ required by fcitx5 API |
| Framework | fcitx5 addon | Only stack with first-class frontends for GNOME Wayland, KDE Wayland and X11 |
| v1 scope | Essentials only | Telex, VNI, Simple Telex, tone placement, auto-restore, spell-check, toggle, tray, install |
| Targets | GNOME Wayland, KDE Wayland, X11 | All first-class in v1; per-target verification required |
| Python | Tooling, corpus scripts, optional future GUI | Never in the keystroke path |
| Name | `directtype` | Language-neutral: the product targets many languages, not only Vietnamese |

### 3.1 Language model

A *language* is a rule pack plus, if needed, an engine. English is the identity engine: every key is committed unchanged, so "English mode" costs nothing and proves the toggle path. Vietnamese is the first real rule pack. The toggle switches the active language for the focused application; per-app memory belongs to `compat`.

Romanization and rule-based languages (e.g. Korean jamo composition) fit this interface. Dictionary-based languages (Chinese, Japanese) do not, and remain out of scope — the name must not imply otherwise.

## 4. Architecture

```
 physical key
      |
 [ fcitx5 ]  frontend: Wayland text-input-v3 / X11 / XWayland
      |
 [ C++ addon ]  addon/engine.cpp
      |  key event -> core
      v
 [ C ABI ]  crates/viet-ffi
      |
 [ Rust core ]  crates/viet-core   (no IME knowledge, no I/O)
      |  returns an *action*, not a side effect
      v
 { consumed, backspaces, commit_text, preedit_text }
      |
 [ C++ strategy layer ]  chooses how the action reaches the app
      |- Preedit          : standard; underlined; universal fallback
      |- SurroundingText  : delete N chars + commit; no underline; needs app support
      '- ForwardKey       : synthetic key events; true UniKey behavior; target-dependent
      |
 per-application profile: auto default + manual override in tray menu
```

Key property: the core is a pure function of (config, buffered word, key event) → action. That makes it testable without fcitx5, and reusable later by an IBus engine, a uinput daemon, or a CLI.

### 4.1 Core API shape

```c
viet_engine_t *viet_engine_new(const viet_config_t *cfg);
void           viet_engine_reset(viet_engine_t *e);
viet_action_t  viet_engine_feed(viet_engine_t *e, uint32_t keysym, uint32_t mods);

typedef struct {
  int      consumed;       /* 1 = do not pass the key through */
  int      backspaces;     /* chars to delete before committing */
  char    *commit_utf8;    /* may be NULL */
  char    *preedit_utf8;   /* may be NULL */
  uint32_t flags;          /* word boundary, invalid word, restored, ... */
} viet_action_t;
```

Rules: no globals, no allocation in the hot path beyond a small reuseable buffer, deterministic output, no dependency on locale.

### 4.2 Output strategies

| Strategy | Underline | Works in | Risk |
|---|---|---|---|
| Preedit | yes | everything | the original complaint |
| SurroundingText | no | GTK3/4, Qt6, parts of Chromium/Electron | needs `surrounding-text` + `delete-surrounding-text`; undo/redo pollution |
| ForwardKey | no | depends on target; good on X11 and KDE | GNOME Wayland feasibility unproven — spike item |

Default policy: try SurroundingText, fall back to Preedit, per-app override; ForwardKey only where verified.

## 5. Repository layout

```
crates/viet-core/     pure Rust engine + tests
crates/viet-cli/      `viet type` for corpus/CI testing
crates/viet-ffi/      extern "C" surface (staticlib)
addon/                C++ fcitx5 addon (CMake, ECM)
  engine.cpp          key handling + strategy layer
  vietaddon.json.in   fcitx5 addon metadata
packaging/debian/     debian/ packaging, PPA recipes
packaging/install/    single-command installer + uninstaller
tests/corpus/         input -> expected output pairs (the main correctness asset)
docs/                 this plan, app compatibility matrix
gui/                  (Phase 5) settings app; v1 uses fcitx5-configtool
```

## 6. Milestones

Estimates assume one developer, part-time. Halve for full-time.

| Phase | Deliverable | Exit criteria | Est. |
|---|---|---|---|
| 0 | Spike: fcitx5 on GNOME/KDE Wayland/X11; measure strategy support per app; test ForwardKey | Written app × strategy matrix; go/no-go on ForwardKey; underline reproduced and explained in each target | 1 wk |
| 1 | `viet-core` + `viet-cli` + corpus | Telex/VNI/Simple Telex correct on ≥2,000 golden cases; auto-restore; tone placement old/new; uppercase | 3–4 wk |
| 2 | fcitx5 addon MVP | Types correctly in GTK3/4, Qt5/6, Firefox, LibreOffice, VS Code, terminal; Preedit + SurroundingText strategies; toggle key; per-app profile | 4–5 wk |
| 3 | Packaging + one-command install | Fresh VM (24.04 and 26.04, amd64 + arm64) → working IME, no manual steps; clean uninstall | 2 wk |
| 4 | Compatibility hardening | User's daily app list passes; ForwardKey where viable; Sublime/terminal/JetBrains fixes | 3 wk |
| 5 | Polish | Optional custom GUI, macros, converter | 3–4 wk |

Total to a shippable v1: **~13–15 weeks part-time**, Phase 0–3 ≈ 10 weeks.

## 7. The one-command install

Target experience:

```
curl -fsSL https://<domain>/install.sh | bash
```

Installer must, idempotently and with a dry-run mode:

1. Add the signed apt repo (amd64 + arm64), `apt install unikey-ime`.
2. `im-config -n fcitx5`; remove IBus autostart.
3. Write session env (`~/.config/environment.d/`) with `GTK_IM_MODULE`, `QT_IM_MODULE`, `XMODIFIERS`; document GTK4/Wayland nuances.
4. Neutralize GNOME's layout switcher: single `us` source, unbind/leave Super+Space free; bind the IME toggle instead.
5. Add autostart; optionally restart the session or print exact logout instructions.
6. `--uninstall` reverses 1–5.

Notes: prefer a signed apt repo over a bare `.deb` for upgrades; a `curl | bash` front end is convenience layered on top, not the only path. Flatpak is not viable for an fcitx5 addon.

## 8. Test matrix

- **Distros:** Ubuntu 24.04, 26.04 (LTS focus).
- **Sessions:** GNOME Wayland, KDE Wayland, X11 (XFCE or GNOME on Xorg).
- **Apps:** Firefox, Chrome/Chromium, GTK3 (gedit), GTK4 (Text Editor), Qt5/Qt6 (Kate), LibreOffice, VS Code, Sublime Text, GNOME Terminal, JetBrains IDE, a Java/Swing app if feasible.
- **Correctness:** golden corpus in CI; property tests (idempotence of restore, tone placement invariants); fuzz the key parser so no input can crash the engine.
- **Packaging:** containerised install test per distro in CI.

## 9. Risks

1. **ForwardKey on GNOME Wayland may be impossible without a privileged helper.** Mitigation: Preedit/SurroundingText cover most apps; scope a privilege-escalated helper only if Phase 0 proves it necessary.
2. **Undo/redo pollution** from delete+commit strategies. Mitigation: prefer surrounding-text, keep commits word-sized, document per-app.
3. **IBus/fcitx5 coexistence** silently breaks the IME. Mitigation: installer config is a first-class, tested artifact.
4. **Shortcut conflicts** (Super+Space, Ctrl+Shift in terminals). Mitigation: one documented toggle, rebindable, no hidden bindings.
5. **Tone-placement edge cases** are the long tail. Mitigation: corpus-first development; never "fix" an edge case without a test.
6. **Name/trademark.** Rename before the first public artifact.

## 10. Open questions

1. Final product name and license (GPL-3.0 vs MIT/Apache for a Rust core).
2. Top 5–8 daily apps, to fix the compatibility priority order.
3. Is a settings GUI needed in v1, or is fcitx5-configtool acceptable?
4. Distribution channel: own apt repo vs PPA vs both.
