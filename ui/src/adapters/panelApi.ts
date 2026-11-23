import { invoke } from "@tauri-apps/api/core";

export type ComponentDto = {
  id: string;
  label: string | null;
  actionKind: "game" | "app";
};

export type RenderView = {
  pages: { components: ComponentDto[] }[];
  currentPage: number;
  locked: boolean;
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
