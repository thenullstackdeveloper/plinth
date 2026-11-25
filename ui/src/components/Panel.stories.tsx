import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import type { RenderView } from "../adapters/panelApi";
import { usePanelStore } from "../store/panelStore";
import { appComponent, component, viewWith } from "../testing/views";
import { Panel } from "./Panel";

const sidebar = [
  appComponent("lock", "Lock", "toggle_lock"),
  appComponent("switch_side", "Switch side", "switch_sidebar"),
  appComponent("quit", "Quit", "quit"),
];
const pages = [[component("world_map", "World map")], [component("bags", "Bags")]];

function showing(overrides: Partial<RenderView>) {
  return () => {
    usePanelStore.setState({ view: viewWith(pages, { sidebar, ...overrides }) });
  };
}

const meta = {
  component: Panel,
  beforeEach: () => {
    usePanelStore.setState(usePanelStore.getInitialState(), true);
  },
} satisfies Meta<typeof Panel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const SidebarLeft: Story = {
  beforeEach: showing({ sidebarSide: "left" }),
  play: async ({ canvas }) => {
    const aside = canvas.getByRole("complementary", { name: "Sidebar" });
    const page = canvas.getByRole("region", { name: "Page" });
    await expect(aside.compareDocumentPosition(page) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  },
};

export const SidebarRight: Story = {
  beforeEach: showing({ sidebarSide: "right" }),
  play: async ({ canvas }) => {
    const aside = canvas.getByRole("complementary", { name: "Sidebar" });
    const page = canvas.getByRole("region", { name: "Page" });
    await expect(aside.compareDocumentPosition(page) & Node.DOCUMENT_POSITION_PRECEDING).toBeTruthy();
  },
};

export const Locked: Story = {
  beforeEach: showing({ locked: true }),
  play: async ({ canvas, canvasElement }) => {
    await expect(canvasElement.querySelector(".panel")).toHaveAttribute("data-locked", "true");
    await expect(canvas.getByRole("button", { name: "Unlock" })).toBeVisible();
  },
};

export const ShowsTheCurrentPage: Story = {
  beforeEach: showing({ currentPage: 1 }),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Bags" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "World map" })).toBeNull();
  },
};
