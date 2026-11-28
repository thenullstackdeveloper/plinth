import type { Page } from "@playwright/test";

import type { ComponentDto, Effects, PageDto, TemplateImages, ThemeVars } from "../src/adapters/panelApi";
import defaultTheme from "../src/testing/fixtures/default-theme.json" with { type: "json" };

/** A game button in the cell `row`, `col`. */
export function game(id: string, label: string, row: number, col: number): ComponentDto {
  return { id, row, col, label, actionKind: "game", appAction: null };
}

/** `component` over `rowSpan` rows and `colSpan` columns from its cell, sent only when more than 1, as Rust does. */
export function spanning(component: ComponentDto, rowSpan: number, colSpan: number): ComponentDto {
  return { ...component, ...(rowSpan > 1 && { rowSpan }), ...(colSpan > 1 && { colSpan }) };
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

/**
 * A page with its grid size, as Rust computes it: the largest `row + rowSpan`
 * and `col + colSpan`, and its `backgroundId` only when it has one.
 */
function withGrid(components: ComponentDto[], backgroundId?: string): PageDto {
  const end = (pick: (c: ComponentDto) => number) => Math.max(0, ...components.map(pick));
  const grid = {
    components,
    rows: end((c) => c.row + (c.rowSpan ?? 1)),
    cols: end((c) => c.col + (c.colSpan ?? 1)),
  };
  return backgroundId === undefined ? grid : { ...grid, backgroundId };
}

export type FakeOptions = {
  sidebarSide?: "left" | "right";
  /** The pages the backend sends, one list of components per page; three pages of two when absent. */
  pages?: ComponentDto[][];
  /** The sidebar buttons, in the order the backend sends them; Lock, Switch side and Quit when absent. */
  sidebar?: ComponentDto[];
  /** The resolved theme the backend sends; the default theme when absent. */
  theme?: ThemeVars;
  /** A component whose press the backend refuses with this error text. */
  failPress?: { componentId: string; error: string };
  /** What `template_images` answers: image id → `data:` URI. None when absent. */
  images?: TemplateImages;
  /** The `backgroundId` of each page, by index, already resolved as Rust does; no background when absent. */
  backgrounds?: (string | undefined)[];
  /** The template's effects; each one absent is its default, `slide` and `lighten`, as Rust resolves it. */
  effects?: Partial<Effects>;
};

/** What the backend holds of its template, and changes on `emitPanelChanged`. */
type TemplateData = {
  pages: PageDto[];
  sidebar: ComponentDto[];
  theme: ThemeVars;
  images: TemplateImages;
  effects: Effects;
};

/** `options` as the page gets them, with the defaults filled in and the grids computed. */
function templateData(options: FakeOptions): TemplateData {
  return {
    pages: (options.pages ?? DEFAULT_PAGES).map((components, i) => withGrid(components, options.backgrounds?.[i])),
    sidebar: options.sidebar ?? SIDEBAR,
    theme: options.theme ?? defaultTheme,
    images: options.images ?? {},
    effects: { pageTransition: "slide", press: "lighten", ...options.effects },
  };
}

/** What the fake recorded, read back with `calls(page)`. */
export type FakeCall = { cmd: string; args: unknown };

/**
 * Installs a stand-in for the Tauri backend before the page loads: the same
 * `window.__TAURI_INTERNALS__` shape `mockIPC` in `@tauri-apps/api/mocks`
 * (2.12.5) fills. It answers the panel commands with its pages (three by
 * default), a lock and a sidebar, like `Panel` in crates/plinth-tauri/src/panel.rs: a refused swipe
 * or press answers with the unchanged view. The pages, the sidebar and the
 * theme are built here, in Node, and handed to the page as an argument.
 * `template_images` answers with `options.images`. The `panel-changed`
 * listeners are kept, so `emitPanelChanged` can change the template and
 * fire the event as the tray does.
 */
export async function installFakeBackend(page: Page, options: FakeOptions = {}): Promise<void> {
  type Injected = Pick<FakeOptions, "sidebarSide" | "failPress"> & { data: TemplateData };
  await page.addInitScript((opts: Injected) => {
    let data = opts.data;
    const state = { currentPage: 0, locked: false, sidebarSide: opts.sidebarSide ?? "right" };
    const calls: { cmd: string; args: unknown }[] = [];

    const view = () => ({
      pages: data.pages,
      currentPage: state.currentPage,
      locked: state.locked,
      sidebar: data.sidebar,
      sidebarSide: state.sidebarSide,
      theme: data.theme,
      // `motion.page-transition` of the default theme.
      pageTransitionMs: 200,
      effects: data.effects,
    });

    const callbacks = new Map<number, (payload: unknown) => void>();
    let nextId = 1;
    /** Callback ids of the `panel-changed` listeners still registered. */
    const listeners = new Set<number>();

    const internals = {
      transformCallback(callback: (payload: unknown) => void) {
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
          case "template_images":
            return data.images;
          case "navigate": {
            const step = args.direction === "next" ? 1 : -1;
            const target = state.currentPage + step;
            if (!state.locked && target >= 0 && target < data.pages.length) state.currentPage = target;
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
          // The event id is the handler's callback id, as `mockIPC` does.
          case "plugin:event|listen": {
            const handler = (args as { handler: number }).handler;
            listeners.add(handler);
            return handler;
          }
          case "plugin:event|unlisten":
            listeners.delete((args as { eventId: number }).eventId);
            return null;
          default:
            throw new Error(`unexpected command ${cmd}`);
        }
      },
    };

    const w = window as unknown as Record<string, unknown>;
    w.__TAURI_INTERNALS__ = internals;
    // `unlisten` in @tauri-apps/api/event drops the callback here first.
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(_event: string, id: number) {
        callbacks.delete(id);
      },
    };
    w.__plinthCalls = calls;
    /** A new template on the backend, page 0 and unlocked, announced like the tray does. */
    w.__plinthChangeTemplate = (next: TemplateData) => {
      data = next;
      state.currentPage = 0;
      state.locked = false;
      for (const id of listeners) callbacks.get(id)?.({ event: "panel-changed", id, payload: null });
    };
  }, {
    sidebarSide: options.sidebarSide,
    failPress: options.failPress,
    data: templateData(options),
  } satisfies Injected);
}

/**
 * Switches the fake backend to the template of `next` (its pages, sidebar,
 * theme, images and effects; the defaults for what it leaves out) and fires
 * `panel-changed`, as picking a template in the tray does.
 */
export async function emitPanelChanged(page: Page, next: FakeOptions = {}): Promise<void> {
  await page.evaluate(
    (data) => (window as unknown as { __plinthChangeTemplate: (d: TemplateData) => void }).__plinthChangeTemplate(data),
    templateData(next),
  );
}

/** Everything the UI asked the fake backend, in order. */
export function calls(page: Page): Promise<FakeCall[]> {
  return page.evaluate(() => (window as unknown as { __plinthCalls: FakeCall[] }).__plinthCalls);
}
