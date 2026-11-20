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

export function renderState(): Promise<RenderView> {
  return invoke<RenderView>("render_state");
}

export function press(componentId: string): Promise<void> {
  return invoke<void>("press", { componentId });
}
