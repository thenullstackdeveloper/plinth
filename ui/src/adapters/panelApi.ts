import { invoke } from "@tauri-apps/api/core";

export type ComponentDto = {
  id: string;
  label: string | null;
  actionKind: "game" | "app";
  /** The app action the component triggers (`"toggle_lock"`, …), or null for a game action. */
  appAction: string | null;
};

export type RenderView = {
  pages: { components: ComponentDto[] }[];
  currentPage: number;
  locked: boolean;
  sidebar: ComponentDto[];
  sidebarSide: "left" | "right";
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
