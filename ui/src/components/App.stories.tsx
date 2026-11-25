import type { Meta, StoryObj } from "@storybook/react-vite";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { expect, mocked, waitFor } from "storybook/test";

import { usePanelStore } from "../store/panelStore";
import { component, viewWith } from "../testing/views";
import { App } from "./App";

const worldMap = component("world_map", "World map");

const meta = {
  component: App,
  beforeEach: () => {
    usePanelStore.setState(usePanelStore.getInitialState(), true);
    mocked(listen).mockResolvedValue(() => {});
    mocked(invoke).mockResolvedValue(viewWith([[worldMap]]));
  },
} satisfies Meta<typeof App>;

export default meta;
type Story = StoryObj<typeof meta>;

export const LoadsTheView: Story = {
  play: async ({ canvas }) => {
    await expect(await canvas.findByRole("button", { name: "World map" })).toBeVisible();
    await expect(invoke).toHaveBeenCalledWith("render_state");
  },
};

export const ShowsTheError: Story = {
  beforeEach: () => {
    mocked(invoke).mockImplementation((cmd: string) =>
      cmd === "render_state" ? Promise.resolve(viewWith([[worldMap]])) : Promise.reject("action: unknown"),
    );
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "World map" }));
    await expect(await canvas.findByRole("alert")).toHaveTextContent("action: unknown");
  },
};

export const ReloadsOnPanelChanged: Story = {
  play: async ({ canvas }) => {
    await canvas.findByRole("button", { name: "World map" });
    await waitFor(() => expect(listen).toHaveBeenCalledWith("panel-changed", expect.any(Function)));
    const handler = mocked(listen).mock.calls[0][1] as () => void;

    mocked(invoke).mockResolvedValue(viewWith([[component("action_1", "Action 1")]]));
    handler();

    await expect(await canvas.findByRole("button", { name: "Action 1" })).toBeVisible();
  },
};
