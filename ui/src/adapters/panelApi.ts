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
  /** The image that fills the button: a key of `fetchTemplateImages()`. Absent when it has none. */
  imageId?: string;
  /** The small image next to the label, or alone: a key of `fetchTemplateImages()` too. Absent when it has none. */
  iconId?: string;
  /** The label is hidden from view but stays the button's name. Absent when it is shown. */
  hideLabel?: boolean;
};

/** Image id (`"t1i0"`) → `data:` URI, for the template on screen. */
export type TemplateImages = Record<string, string>;

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

/** The images of the template on screen, once per template: the view only carries their ids. */
export function fetchTemplateImages(): Promise<TemplateImages> {
  return invoke<TemplateImages>("template_images");
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
