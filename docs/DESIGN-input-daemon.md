# Design: a keyboard-level frontend for Direct Type

Status: proposal, not approved. Nothing here is built.

## Why this exists

The goal in `docs/PLAN.md` §1 is that typing produces the final character immediately —
no composition underline, no "press space to finish". The fcitx5 addon cannot reach that
goal in every application, and 0.0.7 showed why with concrete output: an application may
accept `commitString` and silently ignore `deleteSurroundingText`, so `Đỗ` becomes
`DĐoôỗ`. Chrome's own address bar and page content disagree with each other, which means
no per-application or per-capability rule can be right. The addon therefore defaults to a
preedit, which is correct everywhere and underlined everywhere.

A frontend that sends **key events** does not need the application's cooperation: a
backspace is a backspace to every text field. That is the mechanism this document scopes.

## What it is

A session daemon, not an addon:

```
 physical keyboard
      |
  [ evdev ]  /dev/input/event*   (the daemon takes the device exclusively)
      |
  [ directtype-daemon ]  decode keys -> raw word -> viet-core via the C ABI
      |                                    |
      |                                    '- backspaces + text, as key events
      v
  [ zwp_virtual_keyboard_v1 ]  a virtual keyboard with our own keymap
      |
  [ compositor ]  forwards the events to the focused application
```

Both the deletions and the text travel on **one** event stream from **one** virtual device,
so ordering is guaranteed by construction. That is the property the fcitx5 path cannot
offer: there, a deletion is an optional request to the application.

### Typing arbitrary characters

`EV_KEY` carries keycodes, not Unicode. The virtual keyboard protocol
(`zwp_virtual_keyboard_v1`) lets the client upload its own XKB keymap, so the daemon can
assign the 134 Vietnamese letters to spare keycodes of that keymap and press them. This is
the Linux equivalent of Windows `SendInput` with `KEYEVENTF_UNICODE`, and it is how
`wtype` types non-ASCII text.

## What it costs

- **Privileges.** Reading `/dev/input/event*` needs the `input` group or root; creating a
  uinput/virtual-keyboard device needs access to the compositor's virtual keyboard global.
  A systemd user service with a udev rule, or a small privileged helper. This is the same
  surface `keyd`, `kmonad` and `ydotool` have.
- **It sees every keystroke**, including passwords. The daemon must never log input, and
  the install must say this plainly.
- **One owner of the keyboard.** While the daemon runs, the compositor does not see the
  physical keys; every key, including shortcuts, must be forwarded. A kill switch
  (`Super+Esc` plus a systemd stop) is mandatory, or a bug makes the machine unusable.
- **Layout.** Keycodes must be decoded against the active layout; the safe initial scope
  is a US layout, with the layout detected and reported rather than guessed.
- **It replaces fcitx5 for Vietnamese typing.** fcitx5 keeps working for the applications
  that need it (and for other languages), but the Vietnamese entry has to be off while the
  daemon runs, or both will transform the same keystrokes.

## Phases

1. **Spike (1–2 days).** Read keys from one `evdev` device, write them back unchanged
   through a virtual keyboard with a standard keymap. Prove that shortcuts, modifiers and
   repeat still work, and that the kill switch works. No Vietnamese yet.
2. **Typing (3–5 days).** The engine through the C ABI: word buffer, backspaces and text
   as key events, Vietnamese keymap with the letters the engine can produce, per-word
   correctness checked against the same corpus.
3. **Packaging and safety (2–3 days).** systemd user unit, udev rule, install/uninstall,
   permission check, "no input device" and "another grabber is running" diagnostics, and
   a documented recovery if the daemon misbehaves.
4. **Coexistence (1–2 days).** Turn the fcitx5 Vietnamese entry off while the daemon is
   active, and back on when it stops; a single command to switch frontends.

## Open questions

1. Does the user want a component with this permission profile at all? The alternative is
   to keep the preedit and accept the underline.
2. Wayland only, or X11 too? The virtual keyboard protocol is Wayland; X11 needs XTest,
   a second output path.
3. Per-application behaviour: with a kernel-level frontend every application behaves the
   same, so the per-app profiles in `docs/PLAN.md` become unnecessary — confirm they can
   be dropped.
4. Which applications must be verified before this ships? At minimum the ones that failed
   with the addon: Sublime Text, a terminal, Chrome (page and address bar).
