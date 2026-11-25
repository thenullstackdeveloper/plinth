# Project status and handoff

Snapshot of where Plinth stands and how to continue. Development happens on a Mac, in large
blocks; each finished block is tagged and built on the Windows machine for game testing (see
`docs/flow/2026-11-20-handoff-windows.md`).

**Last updated:** 2026-11-24

## 1. Where the project is

| Area | State |
|---|---|
| Hexagonal core (`plinth-domain`, `plinth-application`, `plinth-infra`) | **Done**, fitness test passing (workspace: 109 Rust tests green on macOS) |
| Template contract (`schema/plinth.template.schema.json`) | Done. Phase 3: a component carries `action` or `keys` (literal keys: `"m"`, `"shift+1"`, or a list of up to 16 steps) |
| SC keybind adapter (`keybinds_sc`) | Done for single-key bindings; chords/joystick unsupported (known limitation) |
| Linux input adapter (`input_uinput`) | Done, validated on real hardware |
| **Windows input adapter (`input_windows`)** | **Done** (`SendInput`), unit-tested, validated against WoW (2026-11-20) |
| **Windows focus spike (`spike_win.exe`)** | **Run against WoW: PASS** (2026-11-20) |
| **`plinth-tauri` (composition root + window)** | Phase 1 focus test with the real window **PASS** (2026-11-20). Phase 2: three-page demo template, `navigate` command, sidebar side remembered in `prefs.json`, `app.quit` and a system tray `Quit`. Phase 3: templates in `%APPDATA%\plinth\templates` (seeded once with `wow-default-keys.yaml` and `wow-demo.yaml`, never overwritten), started with the last chosen one, picked from the tray's `Templates` submenu. Phases 2 and 3 tested together with `v0.3.0` against WoW: **PASS** (2026-11-24), with one open defect (see section 2) |
| React frontend (`ui/`) | Phase 2: swipe between pages (Motion), sidebar on either side with lock, switch side and quit, Zustand store. Phase 3: redraws when the template changes from the tray. Tests: 30 Vitest tests in jsdom, 16 stories in Storybook 10.6.8, run as tests in headless Chromium with axe inside `pnpm --dir ui test` (every component has its stories; `pnpm --dir ui storybook` to browse them) and 8 Playwright e2e |

`main` holds all the work so far (fast-forwarded on 2026-11-24); each new piece of work starts on a short branch from `main`.

## 2. Phase 0 spike — results so far

| Level | Platform | Result |
|---|---|---|
| N1 virtual keyboard emits | Linux | **PASS** — event seen by the compositor in `wev` |
| N2 key reaches a native app | Linux | **PASS** — delivered to the focused window, across workspaces |
| N4 **focus test (R1)** | Windows | **PASS** (2026-11-20) — WoW kept focus while the spike window was clicked, during movement; `M` opened the world map |
| N5 focus test with the real window | Windows | **PASS** (2026-11-20) — WoW kept focus while `World map` was clicked in `plinth-tauri.exe`, during movement; the map opened |
| Phases 2 and 3 (`v0.3.0`) | Windows | **PASS** (2026-11-24) — with the mouse: swipe between pages, a swipe starting on a button changes page without pressing it, the lock blocks swipe and the panel's `Quit`, the side is remembered; every button of `wow-default-keys` works, including `Shift+2`/`Shift+1` and the `[c, b]` macro (both windows open); the templates folder is seeded, the tray's `Templates` switches template, the last choice is remembered, a broken template shows as `(invalid)` and the panel still opens |

Conditions of the N4 run: a single monitor (the spike window floated over WoW in Windowed
(Fullscreen) mode), a **mouse click** rather than a touch, and the spike's default position.
A first run with the default key (`30`, `A`) showed no visible effect: an instant press and
release barely moves the character, so `M` (`50`) is the key to judge by.

**What this means:** both risks are retired on Windows. The *delivery* risk was already retired,
and the *focus* risk (R1) — can the panel be clicked without stealing keyboard focus from the
game? — is retired with `WS_EX_NOACTIVATE`, first with the Win32 spike (N4) and then with the
real Tauri + WebView2 window (N5). Still to confirm: the same behaviour with a real **touch** on
the touchscreen (Windows handles touch input as its own event before turning it into a click).

**Open after the `v0.3.0` test:**

- A swipe only starts on top of a button. Starting from the empty part of the page selects the
  buttons' text instead. The likely cause, not yet confirmed, is that `ui/` has no CSS: the
  draggable area only covers the buttons and nothing prevents text selection.
- The tray is part of the Windows shell. With WoW in Windowed (Fullscreen) the taskbar is
  hidden, so picking a template from the tray means leaving the game and clicking back into it.
  The panel itself still never takes focus.

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

# core.hooksPath is local and NOT inherited by a clone
git config core.hooksPath .githooks
git config user.name  "thenullstackdeveloper"
git config user.email "thenullstackdeveloper@users.noreply.github.com"

cargo test --workspace --features plinth-application/testing   # must be green
./scripts/fitness.sh                                           # must print PASS (bash)
pnpm --dir ui install && pnpm --dir ui test                    # frontend tests
```

On Windows: Rust stable with the MSVC toolchain, Visual Studio 2022 Build Tools ("Desktop
development with C++"), Node 24 LTS and pnpm 10.28.0 (see
`docs/flow/2026-11-20-tauri-panel/windows-build.md`). `scripts/fitness.sh` is a bash script: run
it from Git Bash.

Rust 1.97.1 on the Mac. On macOS, `rustup` comes from Homebrew and its binaries live in
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
1. ~~Minimal `plinth-tauri` and the focus test with the real window~~ — **done, PASS** (2026-11-20, N5).
2. ~~UI: swipe pages, left/right sidebar, lock, quit~~ — **done, PASS** (2026-11-24, with `v0.3.0`).
3. ~~WoW: literal keys and macros in templates, real templates, manual game selection~~ —
   **done, PASS** (2026-11-24, `v0.3.0`). The template with the author's own key
   bindings comes later, by editing a file in the templates folder.
4. **Next:** packaging, docs, license, public repo — end of MVP.
5. Star Citizen as second game: export real keybinds and feed `keybinds_sc`, live data.
6. Linux: uinput + Hyprland, reusing the validated work.

## 8. Non-negotiable rules

See [`roadmap.md`](roadmap.md#non-negotiable-rules). Everything inside the repo is written in
English, except the planning documents under `docs/flow/`; commits follow Conventional Commits
(enforced by `.githooks/commit-msg`).
