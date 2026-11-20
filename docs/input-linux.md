# Input delivery on Linux (uinput, focus, anti-cheat)

> Linux is phase 6 of the [roadmap](roadmap.md#phases). This document keeps the Linux work validated on 2026-06-07 for then.

How Plinth delivers keystrokes to the game on Linux, and why. This is the technical heart of
the project: the risk and the value concentrate here. It documents how things actually work;
the exploratory analysis that led to these conclusions is archived outside the repo.

## uinput via `evdev` (the decision)

Input is injected with **uinput** (a kernel module) through the **`evdev`** crate
(`evdev::uinput::VirtualDevice`). It creates a **kernel-level virtual keyboard**, *below* the
compositor. Decisive advantages:

- Works the same on X11 and **Wayland** (Wayland restricts injection between clients; uinput
  bypasses that because it operates below the display server).
- It is **transparent to Wine/Proton**: the game sees a keyboard indistinguishable from a
  physical one.
- It is **safe with respect to EAC**: it does not inject into the process or read memory; it is
  a virtual HID.

**`enigo` and `rdev` were rejected**: enigo on Linux uses X11/libei (Wayland is experimental and
buggy) and does not use uinput; rdev *sends* via X11. Neither reaches Hyprland+Proton reliably.

## Permissions

- During development: the user needs access to `/dev/uinput` (membership in the `input` group).
- For distribution: a udev rule in `packaging/99-uinput.rules`.

## Emulation level

uinput emits **evdev key codes** (`KEY_A`…), i.e. physical scancodes; the compositor applies the
layout (US, ES, WSAD/ZSQD…) on top. Star Citizen binds by physical key, so this fits. The
translation *SC action → key → evdev code* is the job of the `keybinds_sc` adapter.

> **Current limitation:** `keybinds_sc` resolves single keyboard keys only (`kb1_<key>`).
> Modifier chords (`kb1_lshift+t`) and joystick/mouse bindings are recognized as a valid action
> but resolve to `NotBound` (the sequence model does not yet express simultaneous presses).
> Chord support is a pending improvement.

## Focus: the critical point

A key emitted by uinput reaches **the window with keyboard focus** (like a real keyboard).
Therefore: **the panel must not hold keyboard focus; the game must.** In our favor, Wayland
**separates keyboard focus from pointer/touch focus**: a window can receive touches (you press a
button) without stealing the keyboard.

- **Solution (Hyprland):** the panel is declared with the **`nofocus`** rule (+ float, pin,
  monitor, size). See `docs/hyprland-rules.md`. Placement/focus is delegated to the compositor
  because Tauri's window API **does not work on Wayland** (it controls neither always-on-top nor
  position).
- **Known rough edge** (Hyprland #5618): on multi-monitor setups `nofocus` may shift focus on
  tap. **This is the risk to validate empirically** (see Verification).
- **Robust fallback:** render the panel as a **layer-shell** surface with
  `keyboard_interactivity = none` (via `gtk4-layer-shell`); this is a window concern, the core
  does not change.

## Recommended Star Citizen configuration

- **Borderless windowed**: `USER.cfg` with `r_WindowMode = 1`. It coexists with a panel on the
  second monitor far better than exclusive fullscreen.
- **Avoid gamescope** if possible: it grabs input (`--force-grab-cursor`) and keeps the keyboard;
  a panel outside gamescope may not coexist. Use it only if needed, and test coexistence.

## Anti-cheat (EAC)

Star Citizen uses Easy Anti-Cheat. The dangerous things are **injecting into the process, hooking
graphics, or reading/modifying memory/files** — none of which Plinth does. uinput (a kernel-level
virtual HID) is on the tolerated side; tools like AntiMicroX use it under Proton.
**Non-negotiable rules: only inject input; never touch the game's memory or files.**

## Verification

```bash
cargo test -p plinth-infra -- --ignored     # real uinput test (emits a key; needs /dev/uinput)
```
Tools to inspect: `wev`, `evtest`, `libinput debug-events`, and `hyprctl activewindow` /
`hyprctl clients` to check focus.

**The test that defines viability:** the panel (with the `nofocus` rule) on the 2nd monitor →
tap a button → `hyprctl activewindow` confirms that **SC keeps keyboard focus** → the key reaches
SC. If that passes, the project's technical foundation is validated.
