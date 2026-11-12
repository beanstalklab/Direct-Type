# Capability Map — Vietnamese IME for Ubuntu

Gated artifact: module boundaries and build order must be agreed before any module spec is written.
Once approved, each module gets `docs/SPEC-<module-id>.md`.

| Module id | Responsibility | Depends on |
|---|---|---|
| `engine-core` | Keys → `Action`. Telex/VNI/Simple Telex, tone placement, auto-restore, spell-check. Pure Rust, no I/O, no IME knowledge. | — |
| `ime-addon` | fcitx5 addon: key routing, output strategies (Preedit / SurroundingText / ForwardKey), per-app profile selection, toggle key, tray state. C++ over the C ABI. | `engine-core` |
| `install` | One-command install: apt repo, `.deb`, `im-config`, session env, IBus disable, GNOME shortcut ownership, uninstall. | `ime-addon` |
| `compat` | Per-app verification and workarounds; strategy defaults; escalation to a privileged helper only if the spike proves it necessary. | `ime-addon` |

Build order: `engine-core` → `ime-addon` → `install` → `compat`.

**Exception, deliberate:** the `ime-addon` feasibility spike (see `docs/SPIKE-ime-feasibility.md`) runs *in parallel with* `engine-core`, starting day 1, even though it is listed later. It is investigation, not implementation, and its result can invalidate the product thesis — it must fail fast. `engine-core` was chosen as the first implementation module because it has no platform dependency and no unknowns that code can resolve.

Not modules (deliberately excluded from this map): settings GUI, macro/text expansion, encoding conversion. They are v2 concerns and must not shape v1 interfaces.

## Interfaces at the boundaries

- `engine-core` ↔ `ime-addon`: C ABI in `crates/viet-ffi/include/viet.h`. The core never links against fcitx5.
- `ime-addon` ↔ `install`: the addon ships as a normal fcitx5 addon; the installer only writes config, env and packaging metadata. No private IPC.
- `compat` ↔ `ime-addon`: per-app profiles are addon config entries (fcitx5 `InputMethodGroup`-style), not a separate daemon.
