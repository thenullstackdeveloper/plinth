# Plinth — Roadmap

> **Status:** MVP in progress — Windows first.
> This document holds the vision, the decisions, the phases and the open debates. It is not a
> build plan: each phase is planned one at a time in its own document under `docs/flow/` before
> any code, and tracked as a task on the project board.

---

## Vision

A desktop app that turns a **small ultrawide touchscreen (~12") under the monitor** into a
control panel for PC games: a **touch peripheral**, in the same family as a macro keypad (Razer
Tartarus) or a macro mouse, built to make playing more enjoyable. Similar in spirit to a Stream
Deck, but **built for gaming and deliberately differentiated** (not a clone).

Games: **World of Warcraft first**, **Star Citizen second**.

Differentiators versus the competition (GameGlass, Touch Portal, Macro Deck…):

- **Ultrawide-under-the-monitor** form factor.
- **Game-aware**: templates bind to game actions resolved from the game's keybinds, or to
  literal keys and macros.
- **Tablet-style multi-page** navigation (swipe), not a fixed grid + folders.
- **Configurable sidebar** for right-/left-handed use, with a **lock** against accidental
  page changes.
- **Declarative, community-authored** templates.
- **Free**.

---

## Decisions made

| Topic | Decision |
|---|---|
| **Model** | Personal project + portfolio piece. Free for the community. Not a paid product. |
| **MVP platform** | **Windows.** Development happens on a Mac; testing on a Windows machine. Linux is the last phase, after the MVP release. |
| **MVP game** | **World of Warcraft** (a game the author plays; with literal keys and macros it is useful without parsing its keybinds). |
| **Second game** | **Star Citizen** (a passionate community, tons of keybinds, official live data). Its keybind adapter already exists. |
| **Stack** | **Rust + Tauri**. Frontend in **React + TypeScript** with **Framer Motion** for animations. Systems core in **Rust**. |
| **Template format** | **YAML** as the authoring format, **with a JSON Schema as the contract/source of truth**. |
| **Template bindings** | A button binds to a **semantic game action** (resolved from the game's real keybinds) **or to a literal key or macro** (a finite key sequence). Only keystrokes are representable — never commands or programs. |
| **Game detection (MVP)** | **Manual selection**. Auto-detection is deferred. |
| **Security** | One tap fires one finite key sequence; nothing fires without a human tap. Official game data may be read and shown, never acted on. Never touch the game's memory, never write its files. See [Non-negotiable rules](#non-negotiable-rules). |
| **"Store" v0** | A GitHub repository with Pull Requests. Zero backend. |

### Why this stack (rationale, in brief)

- A React/TS frontend leverages the developer's strengths (fullstack/mobile, TS, React Native,
  Vue). A Rust core deepens the *rare, differentiating* skill for that profile (systems).
- A genuinely **ambitious** Rust+Tauri project (cross-platform input, focus handling) tells a
  story of **depth**, not repetition.
- React opens the natural door to a future mobile version (React → React Native).

### Alternatives discarded

- **gpui-kit** (Longbridge, on Zed's GPUI) as the UI layer — evaluated 2026-11-20 and discarded:
  its documentation confirms neither touch input nor a non-activating window (the one hard
  requirement of the panel), it shipped six releases between 2026-10-12 and 2026-11-16, and it
  means giving up React.

---

## Anti-cheat and fair play

The line is the same for every game: Plinth behaves like a hardware macro device, never like a bot.

**Star Citizen — Easy Anti-Cheat (EAC):**

- ✅ **Allowed**: external input emulation (sending keys). It's what VoiceAttack, Joystick
  Gremlin (listed as compatible), remappers, and a Stream Deck plugin for SC that existed for
  years with this same architecture all do.
- ❌ **Forbidden (ban / login block)**: altering the game's files or reading/editing its memory.

**World of Warcraft:**

- One tap → one finite key sequence: the same model as hardware macro keypads (Razer Tartarus)
  and macro mice, which WoW players have used for years.
- This rests on **practice** — the author's years of using hardware macros in WoW without issue —
  **not on a checked policy text**. Blizzard's own policy on input macros has not been verified
  against a primary source.

---

## Non-negotiable rules

1. **No key without a tap.** Every keystroke comes from a human tap on the panel. One tap fires
   one finite key or key sequence: no loops, no auto-repeat, nothing triggered by game state.
2. **Read and show, never act on readings.** The panel may show data the game itself officially
   exposes (Star Citizen's `Game.log`, official telemetry such as that of Forza or space sims,
   official APIs): shield state, landing gear, tyre wear. A reading may change what the panel
   displays; it never fires an action.
3. **Never touch the game's memory or write its files.** Plinth only injects input. Settings the
   user changes themself — Star Citizen's `USER.cfg`, WoW's display mode — are setup steps, not
   something Plinth does. Reading the game's own keybind configuration read-only is allowed (that
   is how semantic actions are resolved).
4. **Closed catalog: keystrokes only.** Templates may only describe app actions, semantic game
   actions from the catalog, or literal keys and macros. Arbitrary commands or programs are
   unrepresentable. Schema validation is fully offline — the HTTP `$ref` resolvers are disabled
   so a community template can never trigger a network request.

---

## MVP scope

> **Framing.** No game is the core: the core is the game-agnostic desktop base. World of Warcraft
> is the **first vertical** and Star Citizen the second; both sit on top of that generic core and
> reuse all of its work. Keeping the core free of game-specific logic is guaranteed by **interface
> design**, not by build order.
>
> **Center of gravity.** Content is cheap: once the logic exists, adding a button or a template
> is trivial. The real risk and value concentrate in the **input layer**: keeping the touch panel
> from **stealing keyboard focus** from the game while delivering the keystroke. On Windows that
> means a window that never activates (`WS_EX_NOACTIVATE`). That's where the project is won or
> lost.

The MVP is **World of Warcraft on Windows**: phases 0–4 below.

**Included:**

- Engine + declarative format (YAML/Schema) + Windows input-delivery layer (`SendInput`).
- **A single game: World of Warcraft.**
- Template bindings to **literal keys and macros**.
- **Multi-page UI** with swipe (left/right) and pleasant animation.
- **Configurable sidebar** left/right (right-/left-handed).
- **Lock** in the sidebar that blocks page changes (use case: firing abilities in combat without
  accidentally switching pages).
- **Manual** game selection.
- **1–2 real WoW templates** the developer actually uses.
- A borderless, always-on-top window on the touchscreen display, **without stealing focus** from
  the game.
- Packaging, documentation, license and the public repository.

**Excluded (later phases or parked):** Star Citizen and live data (phase 5), Linux (phase 6),
macOS, semantic binding for WoW, everything under [Parked / future](#parked--future).

**Platforms:** Windows first; Linux in phase 6; macOS parked. The engine selects the input-delivery
implementation for the running OS.

---

## Base architecture

Guiding principle: **the core is independent of the UI framework.** If the visual layer changed
tomorrow, the engine stands.

```
UI / render (React + Framer Motion, inside Tauri)
   · swipe between pages · sidebar · lock · animations
   · does NOT send keys: it only describes intent
        │  (Tauri commands / invoke)
        ▼
CORE (Rust)
   · Template validation against JSON Schema (+ clear errors)
   · Closed action catalog  ← security boundary
   · Per-game adapter (action → key, from the game's keybinds)
   · Literal keys/macros (finite key sequences)
   · Per-OS input dispatch (Win / Linux / macOS)
   · Runtime: current page, lock state, toggles, focus handling
```

**Two action categories, separated by design:**

- **Game actions** → a semantic action or a literal key/macro; they send keys to the game through
  the OS input layer.
- **App actions** → navigate pages, toggle the lock, switch the sidebar side. They **never touch
  the game**; they live inside the engine and are safe by definition.

**Live data** (phase 5) is read-only and display-only: it flows from an official source to the UI
and never reaches the input layer.

**On YAML:** kept as the authoring format because it's readable, supports comments, and produces
clean *diffs* in Pull Requests (the v0 store). But the decision that really matters is the
**versioned JSON Schema** as the contract; the surface format is almost a detail.

---

## Phases

The order retires risk first; the flashy parts come after.

| Phase | Content | Platform |
|---|---|---|
| 0 | Focus test (R1) with `spike_win.exe` against WoW, key `M` | Windows |
| 1 | Minimal `plinth-tauri`: borderless, always-on-top, non-activating window with one button wired to `execute_action`; repeats the focus test with the real window | Windows |
| 2 | UI: swipe pages, left/right sidebar, lock | Windows |
| 3 | WoW: literal keys and macros in templates, 1–2 real templates, manual game selection | Windows |
| 4 | Packaging, docs, license, public repo — **end of MVP** | Windows |
| 5 | Star Citizen as second game: semantic actions from its real keybinds, live data | Windows |
| 6 | Linux: uinput + Hyprland, reusing the validated work | Linux |

What each phase proves:

- **0**: Clicking the spike window while WoW has focus leaves WoW as the foreground window, and the key sent (`M`, code 50) opens the world map. The result is recorded in `docs/status.md`.
- **1**: A real Tauri window behaves like the spike: clicking its button keeps WoW focused and the key reaches the game through `execute_action`.
- **2**: Pages change by swipe with animation, the sidebar switches sides and is remembered, and with the lock engaged a swipe does not change page.
- **3**: A template can bind a button to a literal key or key sequence, and the author plays WoW with 1–2 real templates, choosing the game by hand.
- **4**: Someone else can install Plinth on Windows from a published package, the license is decided and in the repo, and the repository is public as the v0 template store.
- **5**: `keybinds_sc` resolves actions from a real exported Star Citizen layout, a real Star Citizen template is in use, and at least one live value from an official source is shown on the panel without ever firing an action.
- **6**: The panel runs on Linux delivering keys through `input_uinput`, and the focus test passes on Hyprland or with the layer-shell fallback.

---

## Parked / future

> Where the months go to die. Documented so it isn't lost, but **not** MVP.

- **Game auto-detection** (by process/window → load the matching template).
- **Visual template editor** (GameGlass "Forge" style).
- **In-app template browser / marketplace**.
- **Semantic binding for WoW**: resolving WoW actions from its keybind files (format unverified).
- **macOS support** for the input layer (`CGEventPost` would need the Accessibility permission).
- **Client-server architecture (the fantasy item):** a server on the PC + a mobile/tablet app
  that reuses the React UI (→ React Native) and sends actions to the server, which fires the
  keystroke on the PC. The current architecture would already allow it (the UI only sends intent;
  that boundary could become a network boundary). **Not a priority:** the priority is the PC with
  the touchscreen.

---

## Open decisions

- **License → DEFERRED, OPEN TO DEBATE.** Postponed on purpose; while the repo is private (and
  even public with no license), *all rights reserved* applies, so the work is already protected
  by default. The owner's stated intent: *"free for the community, but no one profits from my
  work."* That reopens the debate between two paths:
  - **Source-available non-commercial** (e.g. PolyForm Noncommercial): literally reflects that
    wish (free non-commercial use, no third party profiting). **Con:** it is not "open source"
    per the OSI, may deter contributors, and slightly reduces reach as a portfolio piece.
  - **Permissive** (Apache-2.0 / MIT): maximum reading and reuse of the code → a better portfolio
    signal. **Con:** it explicitly allows commercial use by third parties, which is exactly what
    the owner wants to avoid.
  - *Point to settle in the debate:* how much "no one profits" weighs against portfolio reach.
    Decide **before making the repo public** (phase 4; reverting is hard once the community uses
    the code under a permissive license).
- **Domain and trademark.** Verify availability of `plinth` (`.gg` / `.app` / `.io`) and
  trademark registration (USPTO / EUIPO) for software. No apparent conflict with control
  software, but the check is pending.

---

## Competitive reference

- **GameGlass** (gameglass.gg): the closest reference. Freemium + subscription. Its value is the
  "shards" library and the community, not the code. Has a visual editor and a mobile app.
- **Touch Portal / Macro Deck / Elgato Virtual Stream Deck**: variants of a software deck.
- **Corsair Xeneon Edge**: ultrawide-under-the-monitor display hardware → the form factor exists
  in hardware.
- **Razer Tartarus / macro mice**: hardware that sends key sequences on one press → the precedent
  for Plinth's macro model, used for years by WoW players.
- **VoiceAttack / remappers**: heavily used in the SC community → a signal of real appetite for
  input tools in that audience.
