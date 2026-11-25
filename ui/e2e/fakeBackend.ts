import type { Page } from "@playwright/test";

import type { ThemeVars } from "../src/adapters/panelApi";
import defaultTheme from "../src/testing/fixtures/default-theme.json" with { type: "json" };

export type FakeOptions = {
  sidebarSide?: "left" | "right";
  /** The resolved theme the backend sends; the default theme when absent. */
  theme?: ThemeVars;
  /** A component whose press the backend refuses with this error text. */
  failPress?: { componentId: string; error: string };
};

/** What the fake recorded, read back with `calls(page)`. */
export type FakeCall = { cmd: string; args: unknown };

/**
 * Installs a stand-in for the Tauri backend before the page loads: the same
 * `window.__TAURI_INTERNALS__` shape `mockIPC` in `@tauri-apps/api/mocks`
 * (2.12.5) fills. It answers the panel commands with three pages, a lock and a
 * sidebar, like `Panel` in crates/plinth-tauri/src/panel.rs: a refused swipe
 * or press answers with the unchanged view. The theme is resolved here, in
 * Node, and handed to the page as an argument.
 */
export async function installFakeBackend(page: Page, options: FakeOptions = {}): Promise<void> {
  await page.addInitScript((opts: FakeOptions) => {
    type Dto = { id: string; label: string; actionKind: string; appAction: string | null };
    const game = (id: string, label: string): Dto => ({ id, label, actionKind: "game", appAction: null });
    const app = (id: string, label: string, appAction: string): Dto => ({ id, label, actionKind: "app", appAction });

    const pages: Dto[][] = [
      [game("world_map", "World map"), game("character", "Character")],
      [game("bags", "Bags"), game("spellbook", "Spellbook")],
      [game("quest_log", "Quest log"), game("talents", "Talents")],
    ];
    const sidebar = [
      app("lock", "Lock", "toggle_lock"),
      app("switch_side", "Switch side", "switch_sidebar"),
      app("quit", "Quit", "quit"),
    ];
    const state = { currentPage: 0, locked: false, sidebarSide: opts.sidebarSide ?? "right" };
    const calls: { cmd: string; args: unknown }[] = [];

    const view = () => ({
      pages: pages.map((components) => ({ components })),
      currentPage: state.currentPage,
      locked: state.locked,
      sidebar,
      sidebarSide: state.sidebarSide,
      theme: opts.theme,
    });

    const callbacks = new Map<number, unknown>();
    let nextId = 1;

    const internals = {
      transformCallback(callback: unknown) {
        const id = nextId++;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback(id: number) {
        callbacks.delete(id);
      },
      async invoke(cmd: string, args: Record<string, unknown> = {}) {
        calls.push({ cmd, args });
        switch (cmd) {
          case "render_state":
            return view();
          case "navigate": {
            const step = args.direction === "next" ? 1 : -1;
            const target = state.currentPage + step;
            if (!state.locked && target >= 0 && target < pages.length) state.currentPage = target;
            return view();
          }
          case "press": {
            if (opts.failPress && args.componentId === opts.failPress.componentId) {
              throw opts.failPress.error;
            }
            if (args.componentId === "lock") state.locked = !state.locked;
            if (args.componentId === "switch_side") {
              state.sidebarSide = state.sidebarSide === "left" ? "right" : "left";
            }
            return view();
          }
          case "plugin:event|listen":
            return 1;
          case "plugin:event|unlisten":
            return null;
          default:
            throw new Error(`unexpected command ${cmd}`);
        }
      },
    };

    const w = window as unknown as Record<string, unknown>;
    w.__TAURI_INTERNALS__ = internals;
    w.__plinthCalls = calls;
  }, { ...options, theme: options.theme ?? defaultTheme });
}

/** Everything the UI asked the fake backend, in order. */
export function calls(page: Page): Promise<FakeCall[]> {
  return page.evaluate(() => (window as unknown as { __plinthCalls: FakeCall[] }).__plinthCalls);
}
