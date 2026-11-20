# Project status and handoff

Snapshot of where Plinth stands and how to continue. Development happens on a Mac;
game testing happens on a Windows machine.

**Last updated:** 2026-11-20

## 1. Where the project is

| Area | State |
|---|---|
| Hexagonal core (`plinth-domain`, `plinth-application`, `plinth-infra`) | **Done, 31 tests green**, fitness test passing |
| Template contract (`schema/plinth.template.schema.json`) | Done |
| SC keybind adapter (`keybinds_sc`) | Done for single-key bindings; chords/joystick unsupported (known limitation) |
| Linux input adapter (`input_uinput`) | Done, validated on real hardware |
| **Windows input adapter (`input_windows`)** | **Done** (`SendInput`), unit-tested, validated against WoW (2026-11-20) |
| **Windows focus spike (`spike_win.exe`)** | **Run against WoW: PASS** (2026-11-20) |
| `plinth-tauri` (composition root + window) | Not started |
| React frontend | Not started |

Current branch: `feat/phase0-spike`.

## 2. Phase 0 spike — results so far

| Level | Platform | Result |
|---|---|---|
| N1 virtual keyboard emits | Linux | **PASS** — event seen by the compositor in `wev` |
| N2 key reaches a native app | Linux | **PASS** — delivered to the focused window, across workspaces |
| N4 **focus test (R1)** | Windows | **PASS** (2026-11-20) — WoW kept focus while the spike window was clicked, during movement; `M` opened the world map |
| N5 focus test with the real window | Windows | Phase 1, with the real Tauri window |

Conditions of the N4 run: a single monitor (the spike window floated over WoW in Windowed
(Fullscreen) mode), a **mouse click** rather than a touch, and the spike's default position.
A first run with the default key (`30`, `A`) showed no visible effect: an instant press and
release barely moves the character, so `M` (`50`) is the key to judge by.

**What this means:** both risks are retired on Windows. The *delivery* risk was already retired,
and the *focus* risk (R1) — can the panel be clicked without stealing keyboard focus from the
game? — is retired with `WS_EX_NOACTIVATE`. Still to confirm: the same behaviour with a real
**touch** on the touchscreen (Windows handles touch input as its own event before turning it into
a click), and with the real Tauri window (N5, phase 1).

## 3. Running the focus test on Windows

`spike_win.exe` is a self-contained ~150 KB Win32 executable. No Rust, no runtime, no
installer needed on the Windows machine.

### Getting the executable

CI builds it on every push (workflow: `.github/workflows/windows-spike.yml`).

- **From the browser (no tooling):** GitHub → repo → **Actions** → *Windows spike build* →
  latest green run → **Artifacts** → `plinth-spike-win` → downloads a zip containing
  `spike_win.exe`.
- **From a terminal with `gh`:**
  ```bash
  gh run download --repo thenullstackdeveloper/plinth -n plinth-spike-win -D .
  ```
- The workflow can also be triggered by hand from the Actions tab (`workflow_dispatch`).

Note: Windows SmartScreen may warn about an unsigned executable downloaded from the
internet. It is our own build; "More info → Run anyway".

### The test

1. On the Windows machine, set WoW's display mode to **Windowed (Fullscreen)**
   (System → Graphics) so the game runs borderless.
2. Launch World of Warcraft and get in-game.
3. Run the executable, placing the panel on a **second monitor** (arguments are
   `keycode x y width height`):
   ```
   spike_win.exe 50 2200 300
   ```
   `50` is M (world map). Pick a **visible and harmless** action.
4. Focus WoW, then **click the spike window**.

**What to look for** — the window prints, on itself and in the console:

```
focus: <title of the window holding focus>  |  sent scancode 50
```

- **PASS** = focus still reports World of Warcraft **and** the world map opens.
- **FAIL** = focus reports `Plinth`, or nothing happens in game.

The executable reports the foreground window itself (`GetForegroundWindow`), which replaces
`hyprctl activewindow` from the Linux plan — no extra tooling needed.

## 4. Architecture notes for the platform move

### What did not change

The whole core is platform-agnostic and compiles and tests unchanged on macOS:
`plinth-domain`, `plinth-application`, and the `keybinds_sc` / `template_fs` / `config_fs` /
`clock` adapters. This is what the hexagonal boundary bought.

### What was added

| Port | Linux | Windows | macOS |
|---|---|---|---|
| `InputSender` | `input_uinput` (uinput/evdev) | **`input_windows` (`SendInput`)** | not implemented (`CGEventPost` would need Accessibility permission) |
| `WindowPlacement` | no-op + Hyprland rules | not implemented yet | not implemented |

Adding Windows support meant **one new adapter file** and no change to domain or
application code.

**Scancode note:** the domain carries evdev scancodes. For the main key block (letters,
digits, F1–F12, left-hand modifiers) evdev codes and Windows Set-1 scancodes are numerically
identical, so they pass through unchanged. The extended cluster (arrows, navigation, numpad
enter, right-hand modifiers) is E0-prefixed on Windows with different numbering and is **not
translated yet** — `input_windows` returns an error rather than silently sending a wrong key.

**Games run natively on Windows**, so the Wine/Proton layer disappears from the spike
entirely, and `WS_EX_NOACTIVATE` is a documented Win32 mechanism rather than a compositor
quirk — R1 may well be easier here than on Linux.

## 5. Setting up a machine

```bash
git clone git@github.com:thenullstackdeveloper/plinth.git
cd plinth
git switch feat/phase0-spike

# core.hooksPath is local and NOT inherited by a clone
git config core.hooksPath .githooks
git config user.name  "thenullstackdeveloper"
git config user.email "thenullstackdeveloper@users.noreply.github.com"

cargo test --workspace --features plinth-application/testing   # must be green
./scripts/fitness.sh                                           # must print PASS
```

Rust 1.97.1. On macOS, `rustup` comes from Homebrew and its binaries live in
`/opt/homebrew/opt/rustup/bin` — add that to `PATH`.

To type-check the Windows code from the Mac without any C toolchain (`cargo check` does not
link, so no Windows SDK is required):

```bash
rustup target add x86_64-pc-windows-msvc
cargo check -p plinth-infra --target x86_64-pc-windows-msvc
cargo clippy -p plinth-infra --all-targets --target x86_64-pc-windows-msvc -- -D warnings
```

**Not carried by the repo:** the `.scratch/` convention for throwaway notes lives in a
*global* gitignore (`~/.config/git/ignore`); re-create it if you want it on this machine.

## 6. Star Citizen paths on Windows (phase 5)

- Game dir: `C:\Program Files\Roberts Space Industries\StarCitizen\LIVE`
- Borderless: `<LIVE>\USER.cfg` with `r_WindowMode = 1`
- User keybind overrides: `<LIVE>\USER\Client\0\Profiles\default\actionmaps.xml`
- Exported layouts: `<LIVE>\USER\Client\0\Controls\Mappings\layout_<name>_exported.xml`
  (in game: `pp_rebindkeys export all <name>`)

**No keybinds have been exported yet** — `keybinds_sc` has never been fed real data. That
export is the first prerequisite for phase 5 templates.

## 7. Next steps

The phases, in order (see [`roadmap.md`](roadmap.md#phases)):

0. ~~Run the focus test against WoW~~ — **done, PASS** (2026-11-20, section 2).
1. **Next:** minimal `plinth-tauri`: a borderless, always-on-top, non-activating window with one button
   wired to `execute_action`; repeat the focus test with the real window.
2. UI: swipe pages, left/right sidebar, lock.
3. WoW: literal keys and macros in templates, 1–2 real templates, manual game selection.
4. Packaging, docs, license, public repo — end of MVP.
5. Star Citizen as second game: export real keybinds and feed `keybinds_sc`, live data.
6. Linux: uinput + Hyprland, reusing the validated work.

## 8. Non-negotiable rules

See [`roadmap.md`](roadmap.md#non-negotiable-rules). Everything inside the repo is written in
English, except the planning documents under `docs/flow/`; commits follow Conventional Commits
(enforced by `.githooks/commit-msg`).
