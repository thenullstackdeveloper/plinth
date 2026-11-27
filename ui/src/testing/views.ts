import type { ComponentDto, PageDto, RenderView } from "../adapters/panelApi";
import defaultTheme from "./fixtures/default-theme.json";

/** A game component for tests, in the cell `at` (the first one by default). */
export function component(
  id: string,
  label: string | null = id,
  at: { row: number; col: number } = { row: 0, col: 0 },
): ComponentDto {
  return { id, row: at.row, col: at.col, label, actionKind: "game", appAction: null };
}

/** `component` with the image of id `imageId`, a key of the store's `images`. */
export function withImage(component: ComponentDto, imageId: string): ComponentDto {
  return { ...component, imageId };
}

/** An app component (sidebar button) for tests, in `row` of the sidebar. */
export function appComponent(id: string, label: string, appAction: string, row = 0): ComponentDto {
  return { id, row, col: 0, label, actionKind: "app", appAction };
}

/** The components side by side in one row: `row` fixed and `col` 0, 1, 2… in order. */
export function inRow(components: ComponentDto[], row = 0): ComponentDto[] {
  return components.map((c, col) => ({ ...c, row, col }));
}

/** A page with its grid size, as Rust computes it: the largest `row` and `col` plus one. */
function page(components: ComponentDto[]): PageDto {
  const span = (pick: (c: ComponentDto) => number) => Math.max(0, ...components.map((c) => pick(c) + 1));
  return { components, rows: span((c) => c.row), cols: span((c) => c.col) };
}

/** A render view for tests, one entry of `pages` per page, with the default theme. */
export function viewWith(pages: ComponentDto[][], overrides: Partial<RenderView> = {}): RenderView {
  return {
    pages: pages.map(page),
    currentPage: 0,
    locked: false,
    sidebar: [],
    sidebarSide: "right",
    theme: defaultTheme,
    // `motion.page-transition` in default-theme.json.
    pageTransitionMs: 200,
    ...overrides,
  };
}
