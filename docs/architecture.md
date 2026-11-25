# Plinth Architecture

Plinth follows a **hexagonal (ports & adapters)** architecture. The goal: the product logic
(what a panel is, what actions exist, how they are dispatched) must not depend on **any**
external detail — not the UI framework, not the operating system, not the specific game, not
the template format.

> **Status:** the core (`domain` + `application` + `infra`) is implemented and tested. The
> `plinth-tauri` layer (composition root + React UI) is a later phase and does not exist yet.

## Guiding principle

```
plinth-domain        ← (nothing)              pure: product language, game-agnostic
plinth-application   → domain                 use cases + port definitions (traits)
plinth-infra         → application, domain    adapters: uinput, SC keybinds, templates, config…
plinth-tauri (bin)   → infra, application…    composition root + Tauri commands (later phase)
```

Dependencies only point **inward**. The domain knows no one; adapters know the core, never the
other way around. This boundary is **compiler-enforced**: the infrastructure crates aren't even
available to `domain`/`application`, so a `use evdev` / `use tauri` in the core **won't
compile**. `scripts/fitness.sh` additionally checks this in CI by catching infra imports in the
core.

## Layers

### `plinth-domain` — pure, game-agnostic
- **`Action`**: a **closed** enum, `Action::App(AppAction)` | `Action::Game(GameAction)` |
  `Action::Keys(KeyMacro)`. There is no `Raw(String)` variant → running an arbitrary command is
  **unrepresentable** by construction. This is the security boundary expressed as a type.
- **`KeyMacro` / `KeyChord`**: the literal keys a template writes. A chord is a key with the
  modifiers held around it; a macro is 1 to 16 chords fired on one tap.
- **`Template`**: an **immutable** aggregate root (pages, components, sidebar) with its
  invariants (unique IDs, valid page references). An invalid `Template` is unconstructable
  (smart constructors).
- **`RuntimeState`**: the only **mutable** entity (current page, lock, toggles). Invariant:
  navigation is blocked while the lock is engaged; the page index stays in range.
- **`ScanCode` / `KeySequence` / `KeyStep`** (press/hold/toggle + `Duration`): the keystroke
  model, decoupled from any OS subsystem (the values are evdev key codes, but the name and type
  belong to the domain).
- **`ActionCatalog`**: the authority on which actions are legal.

### `plinth-application` — use cases + ports
- **Use cases**: `execute_action`, `query_render_state`, `navigate`, `go_to_page`,
  `toggle_lock`, `switch_sidebar_side`, `load_template`, `refresh_keybinds`.
- **Outbound ports (traits)** — the consumer owns the interface:
  - `InputSender` — emits input events (uinput). *(heavyweight port: OS portability)*
  - `GameKeybindProvider` — resolves `GameAction` → binding by reading the game's config.
    *(heavyweight port: everything game-specific lives here; game #2 = a new impl, the core
    doesn't change)*
  - `TemplateSource` — loads an already-validated template.
  - `Clock` — waits between sequence steps (testable timing).
  - `ConfigStore` — preferences (sidebar side, last template).
  - `WindowPlacement` — window placement (no-op on Linux; real code on Win/Mac, future).
- **DTOs** (`RenderState`) for the UI, and reusable **fakes** behind the `testing` feature.

The heart is `execute_action`: if the action is `App` → it mutates `RuntimeState`; if it is
`Game` → it resolves via `GameKeybindProvider` and emits via `InputSender`. The split by type
guarantees that an app button **never** injects keys and a game button **never** mutates state.

### `plinth-infra` — adapters (the only code that touches the world)
- `input_uinput` — `evdev::uinput::VirtualDevice` → `InputSender` impl.
- `keybinds_sc` — the **only** module that knows about Star Citizen: parses the keybind XML and
  resolves action→key → `GameKeybindProvider` impl.
- `template_fs` — loads YAML + validates against a **JSON Schema** + a mapper that applies the
  catalog.
- `config_fs`, `clock`, `window_placement` (no-op on Linux).

### `plinth-tauri` (binary, later phase)
Composition root: wires the concrete adapters into the use cases and exposes thin
`#[tauri::command]` handlers (deserialize → use case → serialize). The React UI lives beyond the
boundary and **only sends intent, never keys**.

## Two action categories

- **Game actions** → emit keys to the game (go through the OS input layer).
- **App actions** → navigate, toggle the lock, switch the sidebar side. They **never touch the
  game**; they live in the runtime and are safe by definition.

## Security boundary (closed catalog)

Three checks, three places:
1. **Shape (schema)** → infra, in the loader. A versioned JSON Schema is the contract; clear
   errors.
2. **Catalog membership** → domain authority, enforced in the infra mapper: when converting a
   string into an `Action`, an unknown action dies with `UnknownAction`. Because `Action` is a
   closed enum, **an unknown action cannot be constructed** → it cannot reach `execute_action`.
3. **Business invariants** → domain (page references, lock, sequences).

## Game-agnosticism

Guaranteed by the `GameKeybindProvider` port (not by good intentions): `GameAction`s are
**opaque IDs** the domain does not interpret; only the SC adapter knows what they mean. Adding a
second game = a new implementation of the port, without touching the core.

## Testability

- **Domain**: pure unit tests, no ports.
- **Application**: use cases with **fakes** (`FakeInputSender` records sequences,
  `FakeKeybindProvider` a canned table, `FakeClock` deterministic timing,
  `InMemoryTemplateSource`) → the whole dispatch logic is tested **without uinput, without SC,
  without root** (CI-safe).
- **Infra**: the real uinput adapter is covered by an `#[ignore]` integration test (needs
  `/dev/uinput`), run on the developer's machine, not in CI.
- **UI** (`ui/`), three layers, none of them using the network:
  - **Vitest in jsdom** (`pnpm --dir ui test`, project `unit`): the logic without layout, such
    as the store, the swipe maths and the drag guard, plus the existing component tests.
  - **Storybook stories as tests** (same command, project `storybook`): every component in
    `ui/src/components/` has a `*.stories.tsx`, enforced by a unit test. Each story renders in
    headless Chromium, runs its `play` interaction and must pass axe. The Tauri API is mocked in
    `ui/.storybook/preview.ts`, and telemetry is off in `ui/.storybook/main.ts`. Browse the
    stories with `pnpm --dir ui storybook`.
  - **Playwright e2e** (`pnpm --dir ui e2e`): the built app in Chromium at the window's size
    with a fake backend, for layout, real mouse swipes and text selection.

Commands: see `README.md` and `docs/input-linux.md`.

## Quick map

- Dispatch (the heart): `crates/plinth-application/src/usecases.rs`.
- Security boundary: `crates/plinth-domain/src/catalog.rs` and `action.rs`.
- SC adapter: `crates/plinth-infra/src/keybinds_sc.rs`.
- Template contract: `schema/plinth.template.schema.json` + `template_fs.rs`.
