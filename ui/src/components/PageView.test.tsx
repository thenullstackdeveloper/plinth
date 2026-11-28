import { render, screen } from "@testing-library/react";
import { beforeEach, expect, test, vi } from "vitest";

import type { ComponentDto } from "../adapters/panelApi";
import { usePanelStore } from "../store/panelStore";
import { component, inRow, spanning } from "../testing/views";
import { PageView } from "./PageView";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => {
  usePanelStore.setState(usePanelStore.getInitialState(), true);
});

function showing(components: ComponentDto[], rows: number, cols: number) {
  return render(
    <PageView components={components} rows={rows} cols={cols} pageIndex={0} direction={null} locked={false} transitionMs={200} />,
  );
}

/** The grid cells a button covers, as CSS lines: from one more than its `row` and `col`, over its spans. */
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
  expect(cellOf("a")).toEqual(["1 / span 1", "1 / span 1"]);
  expect(cellOf("b")).toEqual(["1 / span 1", "2 / span 1"]);
  expect(cellOf("c")).toEqual(["1 / span 1", "3 / span 1"]);
  expect(cellOf("d")).toEqual(["2 / span 1", "3 / span 1"]);
});

test("places a spanning button across its cells", () => {
  showing(
    [
      spanning(component("tall"), 2, 1),
      spanning(component("wide", "wide", { row: 0, col: 1 }), 1, 3),
      spanning(component("big", "big", { row: 1, col: 1 }), 4, 10),
    ],
    5,
    11,
  );
  expect(cellOf("tall")).toEqual(["1 / span 2", "1 / span 1"]);
  expect(cellOf("wide")).toEqual(["1 / span 1", "2 / span 3"]);
  expect(cellOf("big")).toEqual(["2 / span 4", "2 / span 10"]);
});

test("sizes the grid from the page", () => {
  const { container } = showing(inRow([component("a"), component("b"), component("c")]), 2, 3);
  const surface = container.querySelector<HTMLElement>(".page-surface");
  expect(surface?.style.gridTemplateRows).toBe("repeat(2, minmax(var(--plinth-size-button-min), 1fr))");
  expect(surface?.style.gridTemplateColumns).toBe("repeat(3, minmax(var(--plinth-size-button-min), 1fr))");
});
