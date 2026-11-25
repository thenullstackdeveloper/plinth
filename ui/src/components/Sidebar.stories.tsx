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
