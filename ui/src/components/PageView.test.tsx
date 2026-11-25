import { render, screen } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";

import type { ComponentDto } from "../adapters/panelApi";
import { usePanelStore } from "../store/panelStore";
import { component, inRow } from "../testing/views";
import { PageView } from "./PageView";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  usePanelStore.setState(usePanelStore.getInitialState(), true);
});

function showing(components: ComponentDto[], rows: number, cols: number) {
  return render(
    <PageView components={components} rows={rows} cols={cols} pageIndex={0} direction={null} locked={false} />,
  );
}

/** The grid cell a button sits in, as CSS lines: one more than its `row` and `col`. */
function cellOf(name: string): [string, string] {
  const button = screen.getByRole("button", { name });
  return [button.style.gridRow, button.style.gridColumn];
}

test("places each button in its grid cell", () => {
  showing(
    [...inRow([component("a"), component("b"), component("c")]), component("d", "d", { row: 1, col: 2 })],
    2,
    3,
  );
  expect(cellOf("a")).toEqual(["1", "1"]);
  expect(cellOf("b")).toEqual(["1", "2"]);
  expect(cellOf("c")).toEqual(["1", "3"]);
  expect(cellOf("d")).toEqual(["2", "3"]);
});

test("sizes the grid from the page", () => {
  const { container } = showing(inRow([component("a"), component("b"), component("c")]), 2, 3);
  const surface = container.querySelector<HTMLElement>(".page-surface");
  expect(surface?.style.gridTemplateRows).toBe("repeat(2, minmax(var(--plinth-size-button-min), 1fr))");
  expect(surface?.style.gridTemplateColumns).toBe("repeat(3, minmax(var(--plinth-size-button-min), 1fr))");
});
