import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, test, vi } from "vitest";

import type { RenderView } from "../adapters/panelApi";
import { usePanelStore } from "../store/panelStore";
import { appComponent, component, viewWith } from "../testing/views";
import { Panel } from "./Panel";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

const sidebar = [
  appComponent("lock", "Lock", "toggle_lock"),
  appComponent("switch_side", "Switch side", "switch_sidebar"),
];

function showing(view: RenderView) {
  usePanelStore.setState({ view });
  invokeMock.mockResolvedValue(view);
  return render(<Panel />);
}

beforeEach(() => {
  invokeMock.mockReset();
  usePanelStore.setState(usePanelStore.getInitialState(), true);
});

test("the sidebar goes before the page on the left", () => {
  showing(viewWith([[component("world_map")]], { sidebar, sidebarSide: "left" }));
  const aside = screen.getByRole("complementary", { name: "Sidebar" });
  const page = screen.getByRole("region", { name: "Page" });
  expect(aside.compareDocumentPosition(page) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
});

test("the sidebar goes after the page on the right", () => {
  showing(viewWith([[component("world_map")]], { sidebar, sidebarSide: "right" }));
  const aside = screen.getByRole("complementary", { name: "Sidebar" });
  const page = screen.getByRole("region", { name: "Page" });
  expect(page.compareDocumentPosition(aside) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
});

test("the lock button shows Unlock and the panel is marked while locked", () => {
  const { container } = showing(viewWith([[component("world_map")]], { sidebar, locked: true }));
  const lock = screen.getByRole("button", { name: "Unlock" });
  expect(lock.getAttribute("aria-pressed")).toBe("true");
  expect(container.querySelector('[data-locked="true"]')).not.toBeNull();
});

test("the lock button shows Lock while unlocked", () => {
  showing(viewWith([[component("world_map")]], { sidebar, locked: false }));
  expect(screen.getByRole("button", { name: "Lock" }).getAttribute("aria-pressed")).toBe("false");
});

test("Switch side presses its component", async () => {
  showing(viewWith([[component("world_map")]], { sidebar }));
  await userEvent.click(screen.getByRole("button", { name: "Switch side" }));
  expect(invokeMock).toHaveBeenCalledWith("press", { componentId: "switch_side" });
});
