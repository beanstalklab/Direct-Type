# Spike: is "no underline" achievable?

Owner: _(unassigned)_ · Timebox: **5 working days** · Output: a filled-in results table, a go/no-go on ForwardKey, and a per-app strategy default list.

This spike is investigation only. No product code. Do not start `engine-core` implementation decisions that depend on its outcome.

## Why it exists

The product promise is "types like UniKey: no composition underline, no spacebar to finish a word."
That promise maps to three output strategies with very different reach:

| Strategy | Mechanism | Promise kept? |
|---|---|---|
| Preedit | engine exposes a preedit string; app draws it | No — underlined by design |
| SurroundingText | read text before cursor, delete N chars, commit the new word | Yes, **if** the app supports the required capabilities |
| ForwardKey | swallow key, synthesize key events into the app | Yes, **if** the platform allows injection |

If neither SurroundingText nor ForwardKey works on GNOME Wayland, the product either ships with the underline on the default Ubuntu desktop (the exact complaint) or needs a privileged helper. That is a thesis-level risk, so it is tested first.

## Step 0 — Bootstrap the machine

This host currently has no Rust, no CMake, no pkg-config, no fcitx5 headers. Verify package names before installing:

```bash
apt-cache search fcitx5 | grep -E 'dev|frontend|config'
apt-cache policy extra-cmake-modules wayland-utils
```

Then:

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config cmake extra-cmake-modules \
  fcitx5 fcitx5-config-qt \
  fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt5 fcitx5-frontend-qt6 \
  fcitx5-bamboo wayland-utils
```

Expect `libfcitx5core-dev`, `libfcitx5config-dev`, `libfcitx5utils-dev` (or a single `libfcitx5-dev`) for addon development; adjust to what `apt-cache search` reports.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
rustup component add clippy rustfmt
```

## Step 1 — Get a reference IME running

```bash
im-config -n fcitx5
mkdir -p ~/.config/environment.d
cat > ~/.config/environment.d/ime.conf <<'EOF'
GTK_IM_MODULE=fcitx
QT_IM_MODULE=fcitx
XMODIFIERS=@im=fcitx
EOF
```

Log out and back in (Wayland env vars are read at session start). Then:

```bash
fcitx5-diagnose > /tmp/fcitx5-diagnose.txt 2>&1
fcitx5-remote -o          # ensure fcitx5 owns input
```

`fcitx5-diagnose` output is the single most useful artifact for debugging "the IME silently doesn't load" reports later. Keep it.

Bamboo is installed **only as a reference to observe and measure** — not as a base to fork.

## Step 2 — Confirm the reported bugs, quantitatively

For each app, type `vieejt` + space in Telex and record:

| # | App | Session | Preedit visible? | Underlined? | Text visible before commit? | Notes |
|---|---|---|---|---|---|---|
| 1 | GNOME Text Editor (GTK4) | GNOME Wayland | | | | |
| 2 | gedit (GTK3) | GNOME Wayland | | | | |
| 3 | Firefox | GNOME Wayland | | | | |
| 4 | Chromium/Chrome | GNOME Wayland | | | | |
| 5 | VS Code (Electron) | GNOME Wayland | | | | |
| 6 | Sublime Text | GNOME Wayland | | | | expected: invisible |
| 7 | LibreOffice Writer | GNOME Wayland | | | | |
| 8 | GNOME Terminal | GNOME Wayland | | | | |
| 9 | Kate (Qt6) | KDE Wayland | | | | |
| 10 | Konsole (Qt6) | KDE Wayland | | | | |
| 11 | Firefox | X11 | | | | |
| 12 | xterm | X11 | | | | |

Then repeat rows 1–6 with bamboo's preedit/surrounding-text settings toggled, if it exposes them. Record the exact setting name and value.

## Step 3 — Determine what the platform allows

This decides ForwardKey feasibility. Run under each session type:

```bash
wayland-info | grep -E 'virtual_keyboard|input_method|text_input|input_popup'
```

Record for GNOME Wayland and KDE Wayland:

- `zwp_virtual_keyboard_manager_v1` — required to inject key events
- `zwp_input_method_manager_v2` — required for a standalone (non-addon) IME
- `zwp_text_input_manager_v3` — required for any Wayland IME at all

Also capture the current shortcut ownership, which is the source of the Super+Space complaint:

```bash
gsettings get org.gnome.desktop.input-sources sources
gsettings get org.gnome.desktop.wm.keybindings switch-input-source
gsettings get org.gnome.desktop.wm.keybindings switch-input-source-backward
```

## Step 4 — Verify SurroundingText reach

The cheapest check without writing an addon: in each app, use a text field and observe whether a surrounding-text-capable engine can replace already-typed text without a preedit. Until our own addon exists, use bamboo's surrounding-text option if present; if not, defer this row to the first addon prototype and mark it `unverified`.

Record per app: `surrounding-text` / `delete-surrounding-text` = yes / no / partial.

## Exit criteria

- [ ] Table from Step 2 filled for GNOME Wayland, KDE Wayland, X11
- [ ] `wayland-info` recorded for both Wayland sessions
- [ ] GNOME shortcut ownership recorded, including whether Super+Space is free
- [ ] ForwardKey: **go** / **no-go** / **needs privileged helper**, with evidence
- [ ] A default strategy chosen per app, or the app declared out of scope for v1
- [ ] `docs/PLAN.md` updated if any answer contradicts the current architecture
