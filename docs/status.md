# Project status and handoff

Snapshot of where Plinth stands and how to continue on a different machine.
Written when development moved from the Linux desktop to a Mac (development) with
Windows (testing against Star Citizen).

**Last updated:** 2026-06-07

## 1. Where the project is

| Area | State |
|---|---|
| Hexagonal core (`plinth-domain`, `plinth-application`, `plinth-infra`) | **Done, 31 tests green**, fitness test passing |
| Template contract (`schema/plinth.template.schema.json`) | Done |
| SC keybind adapter (`keybinds_sc`) | Done for single-key bindings; chords/joystick unsupported (known limitation) |
| Linux input adapter (`input_uinput`) | Done, works on real hardware |
| Phase 0 spike | **In progress** — see the results table below |
| `plinth-tauri` (composition root + window) | **Not started** |
| React frontend | **Not started** |

Current branch: `feat/phase0-spike`, branched off `main`.

## 2. Phase 0 spike — results so far (on Linux/Hyprland)

| Level | Result | Notes |
|---|---|---|
| N0 SC borderless | done | `USER.cfg` with `r_WindowMode = 1` created in the LIVE dir |
| N1 uinput emits | **PASS** | Virtual keyboard visible to the compositor; `wev` shows the event |
| N2 native app | **PASS** | Key reaches the focused app, including across workspaces |
| N3 delivery under Wine/Proton | not run | |
| N4 focus test (proxy window) | **not run — this is the decisive test** | |
| N5 focus test (Tauri window) | not run | |

**What this means:** the *delivery* risk is retired — a virtual keyboard reaches whatever
window holds focus. The *focus* risk (R1) is still unproven: nobody has yet verified that
a panel window can be tapped without stealing keyboard focus from the game. **That is the
question the project still depends on.**

## 3. Moving to Mac + Windows

### What does NOT change

This is what the hexagonal architecture bought. Platform is an adapter concern:

- `plinth-domain` — pure, no platform code.
- `plinth-application` — use cases and port definitions.
- `plinth-infra`: `keybinds_sc`, `template_fs`, `config_fs`, `clock` — all portable.

The whole core compiles and tests on macOS and Windows as-is: `evdev` is declared only
under `[target.'cfg(target_os = "linux")'.dependencies]` and `input_uinput` is gated with
`#[cfg(target_os = "linux")]`.

### What DOES change

Two ports need platform implementations. Both were designed as seams from day one:

| Port | Linux (done) | Windows (to build) | macOS |
|---|---|---|---|
| `InputSender` | uinput via `evdev` | `SendInput` (Win32) | `CGEventPost` (needs Accessibility permission) |
| `WindowPlacement` | no-op, Hyprland rules | real code needed | real code needed |

**Windows notes (to verify, not yet tested):**
- Input injection: the `windows` crate exposes `SendInput`. It injects at the OS level,
  like uinput, and is what Stream Deck / AutoHotkey style tools use.
- The focus problem has a native answer on Windows: the extended window style
  **`WS_EX_NOACTIVATE`** marks a window that does not take activation when clicked. This
  is the Windows equivalent of Hyprland's `nofocus`, and it is a documented Win32
  mechanism rather than a compositor quirk — so R1 may well be *easier* on Windows.
  Whether Tauri exposes it or it needs a raw window handle is unverified.
- Star Citizen runs natively on Windows (no Proton/Wine layer), which removes a whole
  class of uncertainty from the spike.

**macOS notes:**
- Star Citizen does not run on macOS. The Mac is a **development** machine only: write
  code, run `cargo test`, build the UI. All game validation happens on Windows.
- If an `InputSender` for macOS is ever built, `CGEventPost` requires the user to grant
  Accessibility permission.

### Redoing the spike on Windows

The design still holds; only the mechanisms change. The levels map like this:

1. **N1** — emit a key with `SendInput` and confirm the OS sees it.
2. **N2** — key reaches a focused native app (Notepad).
3. **N3** — not needed: SC is native on Windows, there is no Wine layer to validate.
4. **N4/N5 (critical)** — a panel window with `WS_EX_NOACTIVATE` on a second monitor:
   click it and confirm SC keeps keyboard focus and the key still arrives.

`crates/plinth-infra/examples/emit_key.rs` is Linux-only by design; the Windows
equivalent will be a sibling example driving the `SendInput` adapter.

## 4. Setting up a new machine

```bash
git clone git@github.com:thenullstackdeveloper/plinth.git
cd plinth
git switch feat/phase0-spike          # spike work lives here

# Enable the commit hook (core.hooksPath is local and NOT inherited by a clone)
git config core.hooksPath .githooks

# Set the commit identity you want for this repo
git config user.name  "thenullstackdeveloper"
git config user.email "thenullstackdeveloper@users.noreply.github.com"

cargo test --workspace --features plinth-application/testing   # must be green
./scripts/fitness.sh                                           # must print PASS
```

Requirements: Rust (developed on 1.97.1). Node 24 / npm only when the frontend starts.
Tauri v2 will need its platform prerequisites (WebView2 on Windows, Xcode CLT on macOS).

**Not carried by the repo:** a personal `.scratch/` directory was used for throwaway
notes, ignored via the *global* gitignore (`~/.config/git/ignore`). Re-create that setting
on the new machine if you want the same convention.

## 5. Star Citizen paths on Windows

The Linux paths in the docs are Lutris-prefix-specific. Native Windows equivalents:

- Game dir: `C:\Program Files\Roberts Space Industries\StarCitizen\LIVE`
- Borderless setting: `<LIVE>\USER.cfg` containing `r_WindowMode = 1`
- User keybind overrides: `<LIVE>\USER\Client\0\Profiles\default\actionmaps.xml`
- Exported layouts: `<LIVE>\USER\Client\0\Controls\Mappings\layout_<name>_exported.xml`
  (export in game with `pp_rebindkeys export all <name>`)

Note that **no keybinds have been exported yet** — `keybinds_sc` has never been fed real
data. Doing that export is the first prerequisite for Phase 3 templates.

## 6. Next steps

1. Build a `SendInput`-based `InputSender` for Windows (new module in `plinth-infra`,
   gated with `#[cfg(target_os = "windows")]`).
2. Re-run the spike levels on Windows, above all the **focus test** — likely via
   `WS_EX_NOACTIVATE`.
3. If focus holds: build minimal `plinth-tauri` (window + one button wired to
   `execute_action`) and confirm again with the real window.
4. Then Phase 2 proper: React UI with swipe pages, sidebar and lock.

## 7. Non-negotiable rules (carry these to any platform)

- Plinth **only injects input**. Never read or write the game's memory or files. The one
  exception is `USER.cfg`, a supported user setting.
- Any future "live data" comes only from safe sources (the game's own `Game.log`, official
  APIs) — never memory reads.
- Templates may only describe actions from the closed catalog; `Action` has no raw/escape
  variant by design, so arbitrary commands are unrepresentable.
- Everything inside the repo is written in English; commits follow Conventional Commits.
