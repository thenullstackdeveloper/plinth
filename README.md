# Plinth

A touchscreen control panel for PC games, made for a small ultrawide touchscreen sitting under
the monitor. Tap a button and Plinth sends its keys to the game, without ever taking the
keyboard away from it.

## What it is

Plinth is a strip of big buttons, in the same family as a macro keypad or a Stream Deck, but
built for gaming:

- **Pages you swipe**, like a tablet, instead of a fixed grid with folders.
- **A sidebar** on the left or the right, for either hand, with a **lock** so a swipe in the
  middle of a fight never changes the page.
- **Templates** decide what every button does: a single key, a chord such as `Shift+1`, or a
  short sequence of keys. A template is a plain text file you can write and share.
- **It never steals focus.** The panel is a borderless 900×300 window that stays on top and is
  never activated, so the game keeps the keyboard while you tap.

The first game is **World of Warcraft**; Star Citizen comes next. Plinth runs on **Windows** and
is free for non-commercial use.

## Install (Windows)

1. Open the repository's **Releases** page on GitHub and download the installer, the file whose
   name ends in `-setup.exe`.
2. Run it. The installer is not signed, so Windows SmartScreen shows *"Windows protected your
   PC"*. Click **More info**, then **Run anyway**.
3. Accept the licence and finish. Plinth installs for your user only, so it does not ask for
   administrator rights. If Microsoft Edge WebView2 is missing (Windows 10 and 11 already have
   it), the installer downloads it.
4. Start **Plinth** from the Start menu.

The installation folder holds `LICENSE` and `THIRD-PARTY-NOTICES.md` next to the program.

**Uninstall** from *Settings → Apps*: find Plinth in the list and choose **Uninstall**. Your
templates and preferences live in `%APPDATA%\plinth`; delete that folder too if you want them
gone.

## Using the panel

The panel opens near the top-left corner of the main display; choosing where it opens is not
supported yet. Run the game in a borderless mode (in World of Warcraft: *System → Graphics →
Display Mode → Windowed (Fullscreen)*), which is how Plinth is tested.

- **Buttons.** A tap sends the button's keys to the window that has the keyboard, which is the
  game. One tap is one finite sequence of keys: nothing repeats and nothing fires on its own.
- **Pages.** Swipe left or right, with a finger or by dragging with the mouse, to change page.
  A swipe can start anywhere on the page, even on a button, without pressing it. At the first
  or last page the panel springs back.
- **Sidebar.**
  - **Lock** stops page changes and the panel's `Quit`; while it is engaged the button reads
    **Unlock**.
  - **Switch side** moves the sidebar to the other edge. Plinth remembers the side.
  - **Quit** closes Plinth (not while locked).
- **Tray icon.** Plinth has an icon in the Windows notification area:
  - **Templates** lists the templates in your folder. Pick one to switch the panel to it; the
    active one has a check mark, and Plinth opens with it next time. A template that does not
    load is shown greyed out as `name (invalid)`.
  - **Quit** always closes Plinth, even while the panel is locked.

With the game in *Windowed (Fullscreen)* the taskbar is hidden, so picking a template from the
tray means leaving the game for a moment. The panel itself never takes focus.

## Templates

Templates are YAML files in **`%APPDATA%\plinth\templates`** (paste that into the Explorer
address bar to open it). The first time Plinth starts, it creates the folder with two templates:

- **`wow-default-keys`** uses World of Warcraft's default keys: the action bar keys `1` to `6`,
  the world map, character, bags, spellbook, quest log and talents, the action bar page
  switches `Shift+2` and `Shift+1`, and one button that opens the character window and the bags
  at once.
- **`wow-demo`** opens the same game windows through Plinth's named World of Warcraft actions.

Plinth never writes to that folder again, so your edits are safe. To get the original two
back, delete the folder and start Plinth.

To write your own, copy a template, give it a new name ending in `.yaml`, edit it in any text
editor and restart Plinth so the tray lists it. A small one:

```yaml
schema_version: "1"
metadata:
  game: "World of Warcraft"
  version: "1"
  author: "you"
pages:
  - components:
      - { id: map, position: { row: 0, col: 0 }, keys: "m", label: "Map" }
      - { id: bar_2, position: { row: 0, col: 1 }, keys: "shift+2", label: "Bar 2" }
      - { id: char_bags, position: { row: 1, col: 0 }, keys: ["c", "b"], label: "Char + bags" }
  - components:
      - { id: talents, position: { row: 0, col: 0 }, action: "wow.talents", label: "Talents" }
sidebar:
  default_side: right
  components:
    - { id: lock, position: { row: 0, col: 0 }, action: "app.toggle_lock", label: "Lock" }
    - { id: switch_side, position: { row: 1, col: 0 }, action: "app.switch_sidebar", label: "Switch side" }
    - { id: quit, position: { row: 2, col: 0 }, action: "app.quit", label: "Quit" }
```

- **A button** has an `id`, a grid `position` and either `keys` or `action`, plus an optional
  `label`.
- **`keys`** is one key (`"m"`), a chord with `shift`, `ctrl` or `alt` (`"shift+1"`), or a list
  of up to 16 of them, sent in order. Keys are letters, digits, `f1` to `f12`, `space`, `enter`,
  `esc`, `tab`, `backspace` and the punctuation keys of the main block; arrows and the numpad
  are not supported yet.
- **`action`** is one of the panel's own actions (`app.next_page`, `app.prev_page`,
  `app.go_to_page:0` for the first page, `app.toggle_lock`, `app.switch_sidebar`, `app.quit`)
  or a named World of Warcraft action (`wow.world_map`, `wow.character`, `wow.bags`,
  `wow.spellbook`, `wow.quest_log`, `wow.talents`), sent with the game's default keys.
- **It has to fit.** With the default sizes a page holds up to 3 rows and 10 columns, and the
  sidebar up to 5 buttons. A template that does not fit the 900×300 panel is rejected, not cut.
- **Colours and sizes** can be changed with an optional `theme` block: see
  [Theming a template](docs/theming.md).

Every field and its rules are in the [template schema](schema/plinth.template.schema.json). A
template that breaks a rule shows as `(invalid)` in the tray and the panel keeps working with
another one. Choosing a template re-reads its file; after fixing an invalid one, restart Plinth.

## What Plinth never does

Plinth behaves like a hardware macro keypad, never like a bot:

1. **No key without a tap.** Every keystroke comes from a person tapping the panel. No loops,
   no auto-repeat, nothing triggered by what happens in the game.
2. **It never acts on what it reads.** The panel may one day show data a game officially
   exposes, but a reading only changes what the panel displays; it never fires an action.
3. **It never touches the game.** Plinth does not read or change the game's memory and does
   not write its files. It only sends keys, as a keyboard would.
4. **Templates can only press keys.** A template can describe panel actions, named game actions
   and literal keys, nothing else: it cannot run a command or a program. Templates are checked
   fully offline, so opening one never makes a network request.

## Status

Version 0.4.0, the end of the minimum viable product: World of Warcraft on Windows. The panel,
its pages, the sidebar, the lock and the templates have been tested on Windows against World of
Warcraft with the mouse; touch with a finger is still to be confirmed. Next come Star Citizen
and then Linux.

- [Project status](docs/status.md): what is done and what has been tested.
- [Roadmap](docs/roadmap.md): the vision, the decisions and the phases.

## Development

Plinth is a Rust workspace with a React and TypeScript interface inside Tauri. On Windows you
need Rust stable with the MSVC toolchain, Visual Studio 2022 Build Tools (*Desktop development
with C++*), Node 24 LTS and pnpm 10.28.0.

```bash
pnpm --dir ui install
cargo test --workspace --features plinth-application/testing   # Rust tests
./scripts/fitness.sh                                           # architecture boundaries (bash)
pnpm --dir ui test                                             # Vitest and the Storybook stories
pnpm --dir ui exec playwright install chromium                 # once, for the e2e tests
pnpm --dir ui e2e                                              # Playwright e2e
pnpm --dir ui storybook                                        # browse the components
cargo test -p plinth-infra -- --ignored                        # real uinput test (Linux, needs /dev/uinput)
```

Run the panel in development (on macOS the keys are only logged):

```bash
pnpm --dir ui dev                # terminal 1: Vite dev server on http://localhost:5173
cargo run -p plinth-tauri        # terminal 2: the panel window
```

Build the installer, on Windows:

```bash
pnpm --dir ui release            # builds the interface, then the NSIS installer
pnpm --dir ui release --no-bundle  # the program only, without an installer
```

The installer ends up in `target/release/bundle/nsis/`.

Before a release, check that the third-party licence notices match the dependencies:

```bash
scripts/third-party-notices.sh --check   # fails if THIRD-PARTY-NOTICES.md is out of date
scripts/third-party-notices.sh           # regenerates it
```

The script needs [cargo-about](https://github.com/EmbarkStudios/cargo-about)
(`cargo install cargo-about --locked`), Node and pnpm.

Commits follow [Conventional Commits](https://www.conventionalcommits.org/). Enable the local
hook that checks them, once per clone:

```bash
git config core.hooksPath .githooks
```

More documentation:

- [Architecture](docs/architecture.md)
- [Input on Linux (uinput, focus, anti-cheat)](docs/input-linux.md)
- [Hyprland rules (panel setup)](docs/hyprland-rules.md)

## Contributing

Plinth is not accepting code contributions for now: its author is shaping the project alone
until it has a solid base. Community templates will be welcome once the template format is
frozen.

## License

Plinth is released under the **PolyForm Noncommercial 1.0.0** licence. In plain words:

- You may use, copy and change Plinth for free for any non-commercial purpose: playing,
  hobby projects, study, research, or inside a charity, school or public institution.
- Nobody may sell Plinth or use it to make money.
- It is source-available, not "open source" in the OSI sense.
- A template you write is your own YAML file, not part of Plinth's code.

The exact terms are in [LICENSE](LICENSE). Plinth includes software by other authors under their
own licences, listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
