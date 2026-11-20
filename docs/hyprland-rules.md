# Hyprland rules for the Plinth panel

> Linux is phase 6 of the [roadmap](roadmap.md#phases). This document keeps the Linux work validated on 2026-06-07 for then.

> On Wayland, Tauri does **not** control window position, "always-on-top", or "no-focus" (see
> `docs/input-linux.md`). Those three are delegated to compositor rules. This document is the
> panel's Linux/Hyprland setup requirement. The `window_placement` adapter in `plinth-infra` is
> a deliberate **no-op**: placement lives here.

## 1. Window rules (`windowrulev2`)

Add to `~/.config/hypr/hyprland.conf`. Adjust `title`/`class` to match the Plinth window and
`monitor`/`size`/`move` to your touchscreen.

```ini
# The panel: floating, pinned, on the touchscreen monitor, fixed size, and NO focus.
windowrulev2 = float,        title:^(Plinth)$
windowrulev2 = pin,          title:^(Plinth)$
windowrulev2 = nofocus,      title:^(Plinth)$
windowrulev2 = monitor 1,    title:^(Plinth)$
windowrulev2 = size 1920 480,title:^(Plinth)$
windowrulev2 = move 0 600,   title:^(Plinth)$
windowrulev2 = noborder,     title:^(Plinth)$
```

`nofocus` is the key piece: the panel receives touches but **never** keyboard focus, so the keys
Plinth injects (via uinput) go to the game window, which keeps focus.

> **Project risk #1 (R1):** on multi-monitor setups, `nofocus` may move workspace focus on tap
> (Hyprland issue #5618). **It must be validated empirically** with the Phase 0 spike (critical
> test: tap the panel → `hyprctl activewindow` confirms the game keeps keyboard focus). If it
> fails, the fallback is to render the panel as a `wlr-layer-shell` surface with
> `keyboard_interactivity = none` (a `gtk4-layer-shell` bridge over wry).

## 2. Game configuration (Star Citizen)

- **Borderless windowed:** in `USER.cfg`, `r_WindowMode = 1`. A normal window coexists with a
  panel on the second monitor far better than exclusive *fullscreen*.
- **Avoid gamescope** if possible: it grabs input and may keep the keyboard. If used, explicitly
  test coexistence with the panel.

## 3. Access to `/dev/uinput`

Plinth creates a virtual keyboard with uinput. That requires access to `/dev/uinput`:

- Be a member of the `input` group, **or**
- install the udev rule from `packaging/99-uinput.rules` (recommended for distribution).
```
