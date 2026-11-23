import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, test, vi } from "vitest";

import { component, viewWith } from "../testing/views";
import { usePanelStore } from "./panelStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

beforeEach(() => {
  invokeMock.mockReset();
  usePanelStore.setState(usePanelStore.getInitialState(), true);
});

test("navigate asks the backend and keeps the view it returns", async () => {
  const before = viewWith([[component("a")], [component("b")]]);
  const after = viewWith([[component("a")], [component("b")]], { currentPage: 1 });
  usePanelStore.setState({ view: before });
  invokeMock.mockResolvedValue(after);

  await usePanelStore.getState().navigate("next");

  expect(invokeMock).toHaveBeenCalledWith("navigate", { direction: "next" });
  expect(usePanelStore.getState().view).toEqual(after);
  expect(usePanelStore.getState().lastDirection).toBe("next");
});

test("a failed press keeps the view and records the error", async () => {
  const view = viewWith([[component("a")]]);
  usePanelStore.setState({ view });
  invokeMock.mockRejectedValue("action: unknown");

  await usePanelStore.getState().press("a");

  expect(usePanelStore.getState().view).toBe(view);
  expect(usePanelStore.getState().error).toBe("action: unknown");
});

test("press keeps the view the backend returns", async () => {
  const before = viewWith([[component("a")]]);
  const after = viewWith([[component("a")]], { locked: true });
  usePanelStore.setState({ view: before });
  invokeMock.mockResolvedValue(after);

  await usePanelStore.getState().press("a");

  expect(invokeMock).toHaveBeenCalledWith("press", { componentId: "a" });
  expect(usePanelStore.getState().view).toEqual(after);
});
