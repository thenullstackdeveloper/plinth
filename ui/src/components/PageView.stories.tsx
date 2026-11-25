import type { Meta, StoryObj } from "@storybook/react-vite";
import { invoke } from "@tauri-apps/api/core";
import { expect, mocked } from "storybook/test";

import { usePanelStore } from "../store/panelStore";
import { component, viewWith } from "../testing/views";
import { PageView } from "./PageView";

const worldMap = component("world_map", "World map");

/** The surface Motion drags: the inner layer of the page. */
function surface(canvasElement: HTMLElement): HTMLElement {
  const el = canvasElement.querySelector<HTMLElement>(".page-surface");
  if (!el) throw new Error("no .page-surface");
  return el;
}

const meta = {
  component: PageView,
  args: { components: [worldMap], pageIndex: 0, direction: null, locked: false },
  beforeEach: () => {
    usePanelStore.setState(usePanelStore.getInitialState(), true);
    mocked(invoke).mockResolvedValue(viewWith([[worldMap]]));
  },
} satisfies Meta<typeof PageView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const PressSendsTheComponent: Story = {
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "World map" }));
    await expect(invoke).toHaveBeenCalledWith("press", { componentId: "world_map" });
  },
};

export const LabelFallsBackToId: Story = {
  args: { components: [component("world_map", null)] },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "world_map" })).toBeVisible();
  },
};

export const Locked: Story = {
  args: { locked: true },
  play: async ({ canvasElement }) => {
    // Motion marks only the element it drags with `touch-action`.
    await expect(surface(canvasElement).style.touchAction).toBe("");
  },
};

export const Unlocked: Story = {
  play: async ({ canvasElement }) => {
    await expect(surface(canvasElement).style.touchAction).toBe("pan-y");
  },
};
