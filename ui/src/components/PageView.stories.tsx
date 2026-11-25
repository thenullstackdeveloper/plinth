import type { Meta, StoryObj } from "@storybook/react-vite";
import { invoke } from "@tauri-apps/api/core";
import { expect, mocked } from "storybook/test";

import { usePanelStore } from "../store/panelStore";
import { component, inRow, viewWith } from "../testing/views";
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
  args: { components: [worldMap], rows: 1, cols: 1, pageIndex: 0, direction: null, locked: false },
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

/** Where a button is drawn, by its accessible name. */
function rectOf(canvasElement: HTMLElement, name: string): DOMRect {
  const button = [...canvasElement.querySelectorAll("button")].find((b) => b.textContent === name);
  if (!button) throw new Error(`no button ${name}`);
  return button.getBoundingClientRect();
}

export const GridPlacement: Story = {
  args: {
    components: [...inRow([component("a"), component("b"), component("c")]), component("d", "d", { row: 1, col: 2 })],
    rows: 2,
    cols: 3,
  },
  play: async ({ canvasElement }) => {
    const [a, b, c, d] = ["a", "b", "c", "d"].map((name) => rectOf(canvasElement, name));
    // The first row, left to right, in three columns.
    await expect(b.top).toBe(a.top);
    await expect(b.left).toBeGreaterThan(a.right);
    await expect(c.left).toBeGreaterThan(b.right);
    // `d` in the second row, under `c`.
    await expect(d.left).toBe(c.left);
    await expect(d.top).toBeGreaterThan(c.bottom);
  },
};
