import type { ComponentDto, RenderView } from "../adapters/panelApi";

/** A game component for tests. */
export function component(id: string, label: string | null = id): ComponentDto {
  return { id, label, actionKind: "game", appAction: null };
}

/** An app component (sidebar button) for tests. */
export function appComponent(id: string, label: string, appAction: string): ComponentDto {
  return { id, label, actionKind: "app", appAction };
}

/** A render view for tests, one entry of `pages` per page. */
export function viewWith(pages: ComponentDto[][], overrides: Partial<RenderView> = {}): RenderView {
  return {
    pages: pages.map((components) => ({ components })),
    currentPage: 0,
    locked: false,
    sidebar: [],
    sidebarSide: "right",
    ...overrides,
  };
}
