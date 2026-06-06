# Plinth — Roadmap

> Roadmap document. Captures the decisions made and the build plan after the analysis
> sessions. **Status:** conception / pre-MVP.
> Fine-grained technical details (interfaces, crates, concrete schema) are settled in the
> analysis preceding each implementation phase, not here.

---

## 1. Vision

A desktop app that turns a **small ultrawide touchscreen (~12") under the monitor** into a
control panel for PC games. Similar in spirit to a Stream Deck, but **built for gaming and
deliberately differentiated** (not a clone).

Differentiators versus the competition (GameGlass, Touch Portal, Macro Deck…):

- **Ultrawide-under-the-monitor** form factor.
- **Game-aware**: templates bind to *game actions*, not physical keys.
- **Tablet-style multi-page** navigation (swipe), not a fixed grid + folders.
- **Configurable sidebar** for right-/left-handed use, with a **lock** against accidental
  page changes.
- **Declarative, community-authored** templates.
- **Free**.

---

## 2. Decisions made

| Topic | Decision |
|---|---|
| **Model** | Personal project + portfolio piece. Free for the community. Not a paid product. |
| **MVP game** | Star Citizen (a real pain point for the author, a passionate community, tons of keybinds). |
| **Stack** | **Rust + Tauri**. Frontend in **React + TypeScript** with **Framer Motion** for animations. Systems core in **Rust**. |
| **Template format** | **YAML** as the authoring format, **with a JSON Schema as the contract/source of truth**. |
| **Game detection (MVP)** | **Manual selection**. Auto-detection is deferred. |
| **Security** | YAML only describes actions from a **closed catalog**. Never run arbitrary commands or code. Never touch the game's files or memory. |
| **"Store" v0** | A GitHub repository with Pull Requests. Zero backend. |

### Why this stack (rationale, in brief)

- A React/TS frontend leverages the developer's strengths (fullstack/mobile, TS, React Native,
  Vue). A Rust core deepens the *rare, differentiating* skill for that profile (systems).
- A genuinely **ambitious** Rust+Tauri project (cross-platform input, focus handling) tells a
  story of **depth**, not repetition.
- React opens the natural door to a future mobile version (React → React Native).

---

## 3. Anti-cheat — validated (key to the project being viable)

Star Citizen uses **Easy Anti-Cheat (EAC)**. The line is clear:

- ✅ **Allowed**: external input emulation (sending keys). It's what VoiceAttack, Joystick
  Gremlin (listed as compatible), remappers, and a Stream Deck plugin for SC that existed for
  years with this same architecture all do.
- ❌ **Forbidden (ban / login block)**: altering the game's files or reading/editing its memory.

**Non-negotiable golden rules:**

1. Plinth **only injects input**. It never touches the game's files or memory.
2. If "live data" is ever shown, **only** from safe sources (e.g. the game's own `Game.log`, or
   official APIs). **Never** memory reads.

> The initial technical spike will confirm in practice, on the developer's machine
> (Linux + Proton), that keystroke delivery reaches SC without issues.

---

## 4. MVP scope

> **Framing.** Star Citizen is **not the core**: the core is the game-agnostic desktop base. SC
> is the **first vertical** and is built first because its adapter sits on top of that generic
> core and **reuses all the work** — going SC-first wastes nothing. Keeping the core free of
> SC-specific logic is guaranteed by **interface design**, not by build order.
>
> **Center of gravity.** Content is cheap: once the logic exists, adding a button or a template
> is trivial. The real risk and value concentrate in the **input layer**: getting the keystroke
> to the game under **Proton/Wine**, and keeping the touch panel from **stealing focus** from
> the game. That's where the project is won or lost.

**Included:**

- Engine + declarative format (YAML/Schema) + input-delivery layer.
- **A single game: Star Citizen.**
- **Semantic binding**: templates reference SC *actions*; the engine resolves them to keys by
  reading the game's real keybind configuration (defaults + user overrides). This neutralizes
  the keybind-drift problem across patches.
- **Multi-page UI** with swipe (left/right) and pleasant animation.
- **Configurable sidebar** left/right (right-/left-handed).
- **Lock** in the sidebar that blocks page changes (use case: assigning shields in combat
  without accidentally switching pages).
- **Manual** game selection.
- **1–2 real SC templates** the developer actually uses.
- A borderless, always-on-top window on the touchscreen display, **without stealing focus** from
  the game.

**OS detection** (Windows/Linux/macOS): the engine detects the system and selects the matching
input-delivery implementation. The MVP prioritizes **Linux** (where the developer dogfoods); the
other platforms are an extension.

---

## 5. Base architecture (high level)

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
   · Per-game adapter (SC: action → key, from the real keybinds)
   · Per-OS input dispatch (Win / Linux / macOS)
   · Runtime: current page, lock state, toggles, focus handling
```

**Two action categories, separated by design:**

- **Game actions** → send keys to the game (closed catalog, go through the OS input layer).
- **App actions** → navigate pages, toggle the lock, switch the sidebar side. They **never touch
  the game**; they live inside the engine and are safe by definition.

**On YAML:** kept as the authoring format because it's readable, supports comments, and produces
clean *diffs* in Pull Requests (the v0 store). But the decision that really matters is the
**versioned JSON Schema** as the contract; the surface format is almost a detail. Since SC
templates can be *generated* from the keybind export, the risk of hand-writing them is small.

---

## 6. Build plan (inside-out)

The order prioritizes retiring risk, not building the flashy parts first.

### Phase 0 — Validation spike
- Input delivery in Rust on Linux: getting a key and a sequence to reach Star Citizen under
  Proton **without stealing focus**.
- It's the biggest technical risk and the project's differentiator, and where the hard work
  concentrates (Proton/Wine + focus handling). Everything else —buttons, templates, pages— is
  trivial by comparison.
- If it comes back green, the rest is execution on familiar ground.

### Phase 1 — Core / engine
- Template JSON Schema (metadata + pages + sidebar + actions).
- Closed action catalog (game + app) and validation with clear errors.
- Star Citizen adapter: resolve action → key from the game's real keybinds.
- Input-delivery layer (at least Linux for the MVP).

### Phase 2 — UI / panel
- Render the template in React.
- Multi-page with swipe (Framer Motion).
- Configurable left/right sidebar.
- A lock that disables swiping.
- Borderless / always-on-top / no-focus window on the touchscreen display.
- Develop with the **mouse** while there's no touchscreen (enough for all development).

### Phase 3 — Real templates + use
- 1–2 real SC templates in actual use (dogfooding).
- Manual game selection.

### Phase 4 — Polish + release
- Packaging, documentation, license.
- Public repository as the "v0 store" (templates via Pull Request).

---

## 7. Parked / future (post-MVP)

> Where the months go to die. Documented so it isn't lost, but **not** MVP.

- **Game auto-detection** (by process/window → load the matching template).
- **Visual template editor** (GameGlass "Forge" style).
- **In-app template browser / marketplace**.
- **Live game data** — only from safe sources (`Game.log`, official APIs).
- **Windows and macOS support** for the input layer (the MVP prioritizes Linux).
- **Client-server architecture (the fantasy item):** a server on the PC + a mobile/tablet app
  that reuses the React UI (→ React Native) and sends actions to the server, which fires the
  keystroke on the PC. The current architecture would already allow it (the UI only sends intent;
  that boundary could become a network boundary). **Not a priority:** the priority is the PC with
  the touchscreen.

---

## 8. Open decisions

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
    Decide **before making the repo public** (reverting is hard once the community uses the code
    under a permissive license).
- **Target OS for v1.** Recommendation: **Linux first** (where dogfooding happens).
- **Domain and trademark.** Verify availability of `plinth` (`.gg` / `.app` / `.io`) and
  trademark registration (USPTO / EUIPO) for software. No apparent conflict with control
  software, but the check is pending.

---

## 9. Competitive reference (brief)

- **GameGlass** (gameglass.gg): the closest reference. Freemium + subscription. Its value is the
  "shards" library and the community, not the code. Has a visual editor and a mobile app.
- **Touch Portal / Macro Deck / Elgato Virtual Stream Deck**: variants of a software deck.
- **Corsair Xeneon Edge**: ultrawide-under-the-monitor display hardware → the form factor exists
  in hardware.
- **VoiceAttack / remappers**: heavily used in the SC community → a signal of real appetite for
  input tools in that audience.
