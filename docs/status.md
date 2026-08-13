# Project status and handoff

Snapshot of where Plinth stands and how to continue. Development happens on a Mac;
Star Citizen testing happens on a Windows machine.

**Last updated:** 2026-08-13

## 1. Where the project is

| Area | State |
|---|---|
| Hexagonal core (`plinth-domain`, `plinth-application`, `plinth-infra`) | **Done, 31 tests green**, fitness test passing |
| Template contract (`schema/plinth.template.schema.json`) | Done |
| SC keybind adapter (`keybinds_sc`) | Done for single-key bindings; chords/joystick unsupported (known limitation) |
| Linux input adapter (`input_uinput`) | Done, validated on real hardware |
| **Windows input adapter (`input_windows`)** | **Done** (`SendInput`), unit-tested, not yet validated against the game |
| **Windows focus spike (`spike_win.exe`)** | **Built by CI, waiting to be run against Star Citizen** |
| `plinth-tauri` (composition root + window) | Not started |
| React frontend | Not started |

Current branch: `feat/phase0-spike`.

## 2. Phase 0 spike — results so far

| Level | Platform | Result |
|---|---|---|
| N1 virtual keyboard emits | Linux | **PASS** — event seen by the compositor in `wev` |
| N2 key reaches a native app | Linux | **PASS** — delivered to the focused window, across workspaces |
| N4/N5 **focus test (R1)** | Windows | **PENDING — this is the decisive test** |

**What this means:** the *delivery* risk is retired. The *focus* risk (R1) — can the panel be
clicked without stealing keyboard focus from the game? — is still unproven, and the whole
project depends on it.

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

1. On the Windows machine, put `USER.cfg` containing `r_WindowMode = 1` in
   `C:\Program Files\Roberts Space Industries\StarCitizen\LIVE\` so SC runs borderless.
2. Launch Star Citizen and get in-game.
3. Run the executable, placing the panel on a **second monitor** (arguments are
   `keycode x y width height`):
   ```
   spike_win.exe 59 2200 300
   ```
   `59` is F1 (mobiGlas). `38` is L (ship lights), `30` is A. Pick a **visible and harmless**
   action — never `v_eject`.
4. Focus Star Citizen, then **click the spike window**.

**What to look for** — the window prints, on itself and in the console:

```
focus: <title of the window holding focus>  |  sent scancode 59
```

- **PASS** = focus still reports Star Citizen **and** the action fires in game.
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

**Star Citizen on Windows is native**, so the Wine/Proton layer disappears from the spike
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

## 6. Star Citizen paths on Windows

- Game dir: `C:\Program Files\Roberts Space Industries\StarCitizen\LIVE`
- Borderless: `<LIVE>\USER.cfg` with `r_WindowMode = 1`
- User keybind overrides: `<LIVE>\USER\Client\0\Profiles\default\actionmaps.xml`
- Exported layouts: `<LIVE>\USER\Client\0\Controls\Mappings\layout_<name>_exported.xml`
  (in game: `pp_rebindkeys export all <name>`)

**No keybinds have been exported yet** — `keybinds_sc` has never been fed real data. That
export is the first prerequisite for Phase 3 templates.

## 7. Next steps

1. **Run the focus test** (section 3). Everything else waits on the result.
2. If it passes: build minimal `plinth-tauri` (borderless window + one button wired to
   `execute_action`) and confirm the same behaviour with the real app window.
3. If it fails: investigate whether Tauri can apply `WS_EX_NOACTIVATE` to its window, or
   fall back to keeping the panel on a separate always-on-top window that never activates.
4. Then Phase 2 proper: React UI with swipe pages, sidebar and lock.
5. Export real SC keybinds and feed `keybinds_sc` (prerequisite for real templates).

## 8. Non-negotiable rules (any platform)

- Plinth **only injects input**. Never read or write the game's memory or files. The one
  exception is `USER.cfg`, a supported user setting.
- Future "live data" comes only from safe sources (the game's own `Game.log`, official APIs)
  — never memory reads.
- Templates may only describe actions from the closed catalog; `Action` has no raw/escape
  variant, so arbitrary commands are unrepresentable. Schema validation is fully offline —
  the HTTP `$ref` resolvers are disabled so a community template can never trigger a network
  request.
- Everything inside the repo is written in English; commits follow Conventional Commits
  (enforced by `.githooks/commit-msg`).
