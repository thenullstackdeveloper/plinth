import type { Meta, StoryObj } from "@storybook/react-vite";
import { invoke } from "@tauri-apps/api/core";
import { expect, mocked } from "storybook/test";

import { usePanelStore } from "../store/panelStore";
import { appComponent, viewWith } from "../testing/views";
import { Sidebar } from "./Sidebar";

const lock = appComponent("lock", "Lock", "toggle_lock");
const switchSide = appComponent("switch_side", "Switch side", "switch_sidebar");
const quit = appComponent("quit", "Quit", "quit");

const meta = {
  component: Sidebar,
  args: { components: [lock, switchSide, quit], locked: false },
  beforeEach: () => {
    usePanelStore.setState(usePanelStore.getInitialState(), true);
    mocked(invoke).mockResolvedValue(viewWith([], { sidebar: [lock, switchSide, quit] }));
  },
} satisfies Meta<typeof Sidebar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Unlocked: Story = {
  beforeEach: () => {
    mocked(invoke).mockResolvedValue(viewWith([], { locked: true, sidebar: [lock, switchSide, quit] }));
  },
  play: async ({ canvas, userEvent }) => {
    const button = canvas.getByRole("button", { name: "Lock" });
    await expect(button).toHaveAttribute("aria-pressed", "false");

    await userEvent.click(button);

    await expect(invoke).toHaveBeenCalledWith("press", { componentId: "lock" });
    await expect(usePanelStore.getState().view?.locked).toBe(true);
  },
};

export const Locked: Story = {
  args: { locked: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Unlock" })).toHaveAttribute("aria-pressed", "true");
    await expect(canvas.getByRole("button", { name: "Quit" })).not.toHaveAttribute("aria-pressed");
  },
};

export const SwitchSidePresses: Story = {
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Switch side" }));
    await expect(invoke).toHaveBeenCalledWith("press", { componentId: "switch_side" });
  },
};

export const QuitPresses: Story = {
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Quit" }));
    await expect(invoke).toHaveBeenCalledWith("press", { componentId: "quit" });
  },
};

export const PressFails: Story = {
  beforeEach: () => {
    mocked(invoke).mockRejectedValue("action: unknown");
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Lock" }));
    await expect(usePanelStore.getState().error).toBe("action: unknown");
  },
};

/** The `<aside>` the story renders. */
function asideOf(canvasElement: HTMLElement): HTMLElement {
  const el = canvasElement.querySelector<HTMLElement>(".sidebar");
  if (!el) throw new Error("no .sidebar");
  return el;
}

export const ColumnWidth: Story = {
  play: async ({ canvasElement }) => {
    const aside = asideOf(canvasElement);
    // `size.sidebar.width` 128px and `color.sidebar.background` #191b26 in default-theme.json.
    await expect(aside.getBoundingClientRect().width).toBe(128);
    await expect(getComputedStyle(aside).backgroundColor).toBe("rgb(25, 27, 38)");
    // Behind it, the panel takes `color.panel.background` #11121a.
    const panel = aside.closest<HTMLElement>(".panel");
    await expect(panel && getComputedStyle(panel).backgroundColor).toBe("rgb(17, 18, 26)");
  },
};

export const ButtonsShareTheHeight: Story = {
  play: async ({ canvasElement }) => {
    const aside = asideOf(canvasElement).getBoundingClientRect();
    const buttons = [...canvasElement.querySelectorAll<HTMLElement>(".sidebar > button")];
    const rects = buttons.map((b) => b.getBoundingClientRect());
    await expect(rects).toHaveLength(3);
    // The column fills the panel's height: one column of equal buttons, from
    // the sidebar's padding (8px) at the top to its padding at the bottom,
    // with `space.sidebar.gap` (8px) between them.
    await expect(aside.height).toBe(300);
    const padding = 8;
    const gap = 8;
    const share = (aside.height - 2 * padding - 2 * gap) / 3;
    for (const [i, rect] of rects.entries()) {
      await expect(rect.height).toBeCloseTo(share, 1);
      await expect(rect.top).toBeCloseTo(aside.top + padding + i * (share + gap), 1);
      await expect(rect.width).toBeCloseTo(aside.width - 2 * padding, 1);
    }
    // Never shorter than a finger: `size.touch.min` 44px.
    for (const button of buttons) await expect(getComputedStyle(button).minHeight).toBe("44px");
  },
};
