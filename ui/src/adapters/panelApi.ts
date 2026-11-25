import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type ComponentDto = {
  id: string;
  /** The component's cell, zero-based. */
  row: number;
  col: number;
  label: string | null;
  actionKind: "game" | "app" | "keys";
  /** The app action the component triggers (`"toggle_lock"`, …), or null for game and key buttons. */
  appAction: string | null;
};

/** CSS custom property → value (`"--plinth-color-button-background"` → `"#262a3b"`), resolved by Rust. */
export type ThemeVars = Record<string, string>;

/** One page: its components and the size of its grid (the largest `row` and `col` plus one). */
export type PageDto = {
  components: ComponentDto[];
  rows: number;
  cols: number;
};

export type RenderView = {
  pages: PageDto[];
  currentPage: number;
  locked: boolean;
  sidebar: ComponentDto[];
  sidebarSide: "left" | "right";
  theme: ThemeVars;
  /** How long the change of page takes, from `motion.page-transition`. */
  pageTransitionMs: number;
};

export type NavDirection = "next" | "prev";

export function renderState(): Promise<RenderView> {
  return invoke<RenderView>("render_state");
}

export function press(componentId: string): Promise<RenderView> {
  return invoke<RenderView>("press", { componentId });
}

export function navigate(direction: NavDirection): Promise<RenderView> {
  return invoke<RenderView>("navigate", { direction });
}

/** Sent by Rust when the panel changes outside a command (a template picked in the tray). */
export const PANEL_CHANGED_EVENT = "panel-changed";

/** Calls `handler` on every panel change; resolves to the function that stops listening. */
export function onPanelChanged(handler: () => void): Promise<() => void> {
  return listen(PANEL_CHANGED_EVENT, () => handler());
}
