import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, test, vi } from "vitest";

import type { ComponentDto, RenderView } from "../adapters/panelApi";
import { App } from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function viewWith(components: ComponentDto[]): RenderView {
  return { pages: [{ components }], currentPage: 0, locked: false };
}

const worldMap: ComponentDto = { id: "world_map", label: "World map", actionKind: "game" };

function backend(view: RenderView, onPress: () => Promise<unknown> = () => Promise.resolve()) {
  invokeMock.mockImplementation((command: string) => {
    if (command === "render_state") return Promise.resolve(view);
    if (command === "press") return onPress();
    return Promise.reject(new Error(`unexpected command ${command}`));
  });
}

beforeEach(() => {
  invokeMock.mockReset();
});

test("renders the buttons of the current page", async () => {
  backend(viewWith([worldMap]));
  render(<App />);
  expect(await screen.findByRole("button", { name: "World map" })).toBeTruthy();
});

test("pressing a button calls press with its id", async () => {
  backend(viewWith([worldMap]));
  render(<App />);
  await userEvent.click(await screen.findByRole("button", { name: "World map" }));
  expect(invokeMock).toHaveBeenCalledWith("press", { componentId: "world_map" });
});

test("falls back to the id when there is no label", async () => {
  backend(viewWith([{ ...worldMap, label: null }]));
  render(<App />);
  expect(await screen.findByRole("button", { name: "world_map" })).toBeTruthy();
});

test("shows the error when press fails", async () => {
  backend(viewWith([worldMap]), () => Promise.reject("action: unknown"));
  render(<App />);
  await userEvent.click(await screen.findByRole("button", { name: "World map" }));
  expect((await screen.findByRole("alert")).textContent).toBe("action: unknown");
});
