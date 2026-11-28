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

/** `component` over `rowSpan` rows and `colSpan` columns from its cell, sent only when more than 1, as Rust does. */
export function spanning(component: ComponentDto, rowSpan: number, colSpan: number): ComponentDto {
  return { ...component, ...(rowSpan > 1 && { rowSpan }), ...(colSpan > 1 && { colSpan }) };
}

/** `component` with the image of id `imageId`, a key of the store's `images`. */
export function withImage(component: ComponentDto, imageId: string): ComponentDto {
  return { ...component, imageId };
}

/** `component` with the icon of id `iconId`, a key of the store's `images`. */
export function withIcon(component: ComponentDto, iconId: string): ComponentDto {
  return { ...component, iconId };
}

/** `component` with its label hidden from view; it stays the button's name. */
export function hidingLabel(component: ComponentDto): ComponentDto {
  return { ...component, hideLabel: true };
}

/** An app component (sidebar button) for tests, in `row` of the sidebar. */
export function appComponent(id: string, label: string, appAction: string, row = 0): ComponentDto {
  return { id, row, col: 0, label, actionKind: "app", appAction };
}

/** The components side by side in one row: `row` fixed and `col` 0, 1, 2… in order. */
export function inRow(components: ComponentDto[], row = 0): ComponentDto[] {
  return components.map((c, col) => ({ ...c, row, col }));
}

/** A page with its grid size, as Rust computes it: the largest `row + rowSpan` and `col + colSpan`. */
function page(components: ComponentDto[]): PageDto {
  const end = (pick: (c: ComponentDto) => number) => Math.max(0, ...components.map(pick));
  return {
    components,
    rows: end((c) => c.row + (c.rowSpan ?? 1)),
    cols: end((c) => c.col + (c.colSpan ?? 1)),
  };
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
