import { create } from "zustand";

import * as panelApi from "../adapters/panelApi";
import type { NavDirection, RenderView } from "../adapters/panelApi";

type PanelState = {
  /** The view the backend last returned: the single source of truth. */
  view: RenderView | null;
  error: string | null;
  /** Direction of the last page change, for the page animation. */
  lastDirection: NavDirection | null;
  load(): Promise<void>;
  press(componentId: string): Promise<void>;
  navigate(direction: NavDirection): Promise<void>;
};

export const usePanelStore = create<PanelState>()((set, get) => ({
  view: null,
  error: null,
  lastDirection: null,

  load: async () => {
    try {
      set({ view: await panelApi.renderState(), error: null });
    } catch (reason) {
      set({ error: String(reason) });
    }
  },

  press: async (componentId) => {
    try {
      set({ view: await panelApi.press(componentId), error: null });
    } catch (reason) {
      set({ error: String(reason) });
    }
  },

  navigate: async (direction) => {
    const before = get().view?.currentPage;
    try {
      const view = await panelApi.navigate(direction);
      set(view.currentPage !== before ? { view, error: null, lastDirection: direction } : { view, error: null });
    } catch (reason) {
      set({ error: String(reason) });
    }
  },
}));
