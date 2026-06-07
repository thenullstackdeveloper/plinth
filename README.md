# Plinth

Desktop control panel for PC games on a **touchscreen** (an *ultrawide* unit sitting under
the monitor). Buttons and actions are configurable through **declarative templates**,
*game-aware*, and free for the community.

> **Status:** MVP in progress. The hexagonal core (domain + use cases + adapters) is
> implemented and tested; the Tauri/React layer and the Phase 0 input-validation spike (on
> the developer's machine) are still pending.

## Idea

Similar in spirit to a Stream Deck, but built for gaming and deliberately differentiated:
tablet-style swipeable multi-page navigation, a configurable sidebar for right-/left-handed
use with a lock that prevents accidental page changes, and templates bound to *game actions*
(not physical keys), resolved from the game's real keybind configuration. First vertical:
**Star Citizen**.

## Stack

- **Core:** Rust
- **UI:** Tauri + React + TypeScript (Framer Motion)
- **Input delivery (Linux):** uinput, via the `evdev` crate

## Architecture

Cargo workspace with **compiler-enforced boundaries** (ports & adapters):

```
plinth-domain        ← (nothing)              pure: product language, game-agnostic
plinth-application   → domain                 use cases + port definitions (traits)
plinth-infra         → application, domain    adapters: uinput, SC keybinds, templates, config
plinth-tauri (bin)                            composition root + UI (later phase, not present yet)
```

An infrastructure crate (`evdev`, `serde`…) used from the core **won't compile**;
`scripts/fitness.sh` also enforces this in CI.

## Development

```bash
cargo test --workspace --features plinth-application/testing   # tests (CI-safe)
cargo test -p plinth-infra -- --ignored                        # real uinput test (needs /dev/uinput)
./scripts/fitness.sh                                           # architectural fitness test
```

Commits follow [Conventional Commits](https://www.conventionalcommits.org/). Enable
the local hook that enforces it (once per clone):

```bash
git config core.hooksPath .githooks
```

## Documentation

- [Architecture](docs/architecture.md)
- [Roadmap](docs/roadmap.md)
- [Input on Linux (uinput, focus, anti-cheat)](docs/input-linux.md)
- [Hyprland rules (panel setup)](docs/hyprland-rules.md)

## License

Undecided for now. **All rights reserved.**
