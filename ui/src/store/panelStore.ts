import { create } from "zustand";

import * as panelApi from "../adapters/panelApi";
import type { NavDirection, RenderView, TemplateImages } from "../adapters/panelApi";
import { safeImageSrc } from "../image";

type PanelState = {
  /** The view the backend last returned: the single source of truth. */
  view: RenderView | null;
  /** Image id → `data:` URI of the template on screen, only the URIs `safeImageSrc` lets through. */
  images: TemplateImages;
  error: string | null;
  /** Direction of the last page change, for the page animation. */
  lastDirection: NavDirection | null;
  load(): Promise<void>;
  press(componentId: string): Promise<void>;
  navigate(direction: NavDirection): Promise<void>;
};

/**
 * The template's images that pass `safeImageSrc`. If the backend cannot send
 * them, the panel still loads, without images.
 */
async function fetchSafeImages(): Promise<TemplateImages> {
  const images: TemplateImages = {};
  try {
    const sent: unknown = await panelApi.fetchTemplateImages();
    if (typeof sent !== "object" || sent === null) return images;
    for (const [id, uri] of Object.entries(sent)) {
      const safe = safeImageSrc(uri);
      if (safe !== null) images[id] = safe;
    }
  } catch {
    // No images: the buttons show their labels.
  }
  return images;
}

export const usePanelStore = create<PanelState>()((set, get) => ({
  view: null,
  images: {},
  error: null,
  lastDirection: null,

  // The images first, so the view never names an image the table does not have yet.
  load: async () => {
    const images = await fetchSafeImages();
    try {
      set({ view: await panelApi.renderState(), images, error: null });
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
