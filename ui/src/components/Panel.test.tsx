import { invoke } from "@tauri-apps/api/core";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, test, vi } from "vitest";

import type { RenderView } from "../adapters/panelApi";
import { usePanelStore } from "../store/panelStore";
import { appComponent, component, viewWith } from "../testing/views";
import panelCss from "../panel.css?raw";
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

/** The declarations of every `panel.css` rule whose selector list has `selector`. */
function rule(selector: string): string {
  const css = panelCss.replace(/\/\*[\s\S]*?\*\//g, "");
  return Array.from(css.matchAll(/([^{}]+)\{([^}]*)\}/g))
    .filter(([, selectors]) => selectors.split(",").some((s) => s.trim() === selector))
    .map(([, , body]) => body)
    .join("\n");
}

test("the page sits beside the sidebar and fills the panel", () => {
  const { container } = showing(viewWith([[component("world_map")]], { sidebar }));
  const panel = container.firstElementChild as HTMLElement;
  expect(panel.classList).toContain("panel");
  expect(screen.getByRole("region", { name: "Page" }).classList).toContain("page");
  expect(rule(".panel")).toMatch(/display:\s*flex/);
  expect(rule(".page")).toMatch(/flex:\s*1/);
});

test("the surface that drags is the one stretched over the whole page", () => {
  showing(viewWith([[component("world_map")]], { sidebar }));
  const page = screen.getByRole("region", { name: "Page" });
  // Motion marks the element it drags with `touch-action: pan-y`.
  const draggable = Array.from(page.querySelectorAll<HTMLElement>("div")).filter(
    (el) => el.style.touchAction === "pan-y",
  );
  expect(draggable).toHaveLength(1);
  expect(draggable[0].classList).toContain("page-surface");
  expect(draggable[0].parentElement?.classList).toContain("page-slide");
  expect(rule(".page-surface")).toMatch(/height:\s*100%/);
  expect(rule(".page-slide")).toMatch(/height:\s*100%/);
});

test("nothing in the panel can be selected as text", () => {
  // The standard property (WebView2 on Windows) and the prefixed one (WebKit).
  expect(rule("body")).toMatch(/(^|[\s;])user-select:\s*none/);
  expect(rule("body")).toMatch(/-webkit-user-select:\s*none/);
  expect(rule("#root")).toMatch(/height:\s*100%/);
});
