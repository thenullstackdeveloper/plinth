import type { Page } from "@playwright/test";

import type { ComponentDto, PageDto, ThemeVars } from "../src/adapters/panelApi";
import defaultTheme from "../src/testing/fixtures/default-theme.json" with { type: "json" };

/** A game button in the cell `row`, `col`. */
export function game(id: string, label: string, row: number, col: number): ComponentDto {
  return { id, row, col, label, actionKind: "game", appAction: null };
}

/** An app button (a sidebar one) in `row` of its column. */
export function app(id: string, label: string, appAction: string, row: number): ComponentDto {
  return { id, row, col: 0, label, actionKind: "app", appAction };
}

/** Three pages of two buttons side by side. */
const DEFAULT_PAGES: ComponentDto[][] = [
  [game("world_map", "World map", 0, 0), game("character", "Character", 0, 1)],
  [game("bags", "Bags", 0, 0), game("spellbook", "Spellbook", 0, 1)],
  [game("quest_log", "Quest log", 0, 0), game("talents", "Talents", 0, 1)],
];

const SIDEBAR: ComponentDto[] = [
  app("lock", "Lock", "toggle_lock", 0),
  app("switch_side", "Switch side", "switch_sidebar", 1),
  app("quit", "Quit", "quit", 2),
];

/** A page with its grid size, as Rust computes it: the largest `row` and `col` plus one. */
function withGrid(components: ComponentDto[]): PageDto {
  const span = (pick: (c: ComponentDto) => number) => Math.max(0, ...components.map((c) => pick(c) + 1));
  return { components, rows: span((c) => c.row), cols: span((c) => c.col) };
}

export type FakeOptions = {
  sidebarSide?: "left" | "right";
  /** The pages the backend sends, one list of components per page; three pages of two when absent. */
  pages?: ComponentDto[][];
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
 * (2.12.5) fills. It answers the panel commands with its pages (three by
 * default), a lock and a sidebar, like `Panel` in crates/plinth-tauri/src/panel.rs: a refused swipe
 * or press answers with the unchanged view. The pages, the sidebar and the
 * theme are built here, in Node, and handed to the page as an argument.
 */
export async function installFakeBackend(page: Page, options: FakeOptions = {}): Promise<void> {
  type Injected = Omit<FakeOptions, "pages"> & { pages: PageDto[]; sidebar: ComponentDto[] };
  await page.addInitScript((opts: Injected) => {
    const { pages, sidebar } = opts;
    const state = { currentPage: 0, locked: false, sidebarSide: opts.sidebarSide ?? "right" };
    const calls: { cmd: string; args: unknown }[] = [];

    const view = () => ({
      pages,
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
  }, {
    ...options,
    pages: (options.pages ?? DEFAULT_PAGES).map(withGrid),
    sidebar: SIDEBAR,
    theme: options.theme ?? defaultTheme,
  } satisfies Injected);
}

/** Everything the UI asked the fake backend, in order. */
export function calls(page: Page): Promise<FakeCall[]> {
  return page.evaluate(() => (window as unknown as { __plinthCalls: FakeCall[] }).__plinthCalls);
}
