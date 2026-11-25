import type { Meta, StoryObj } from "@storybook/react-vite";
import { invoke } from "@tauri-apps/api/core";
import { type ComponentProps, useState } from "react";
import { expect, mocked, waitFor } from "storybook/test";

import { usePanelStore } from "../store/panelStore";
import { component, inRow, viewWith } from "../testing/views";
import { PageView } from "./PageView";

const worldMap = component("world_map", "World map");

/** The surface Motion drags: the inner layer of the page. */
function surface(canvasElement: HTMLElement): HTMLElement {
  const el = canvasElement.querySelector<HTMLElement>(".page-surface");
  if (!el) throw new Error("no .page-surface");
  return el;
}

const meta = {
  component: PageView,
  args: { components: [worldMap], rows: 1, cols: 1, pageIndex: 0, direction: null, locked: false, transitionMs: 200 },
  beforeEach: () => {
    usePanelStore.setState(usePanelStore.getInitialState(), true);
    mocked(invoke).mockResolvedValue(viewWith([[worldMap]]));
  },
} satisfies Meta<typeof PageView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const PressSendsTheComponent: Story = {
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "World map" }));
    await expect(invoke).toHaveBeenCalledWith("press", { componentId: "world_map" });
  },
};

export const LabelFallsBackToId: Story = {
  args: { components: [component("world_map", null)] },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "world_map" })).toBeVisible();
  },
};

export const Locked: Story = {
  args: { locked: true },
  play: async ({ canvasElement }) => {
    // Motion marks only the element it drags with `touch-action`.
    await expect(surface(canvasElement).style.touchAction).toBe("");
  },
};

export const Unlocked: Story = {
  play: async ({ canvasElement }) => {
    await expect(surface(canvasElement).style.touchAction).toBe("pan-y");
  },
};

/** Where a button is drawn, by its accessible name. */
function rectOf(canvasElement: HTMLElement, name: string): DOMRect {
  const button = [...canvasElement.querySelectorAll("button")].find((b) => b.textContent === name);
  if (!button) throw new Error(`no button ${name}`);
  return button.getBoundingClientRect();
}

export const GridPlacement: Story = {
  args: {
    components: [...inRow([component("a"), component("b"), component("c")]), component("d", "d", { row: 1, col: 2 })],
    rows: 2,
    cols: 3,
  },
  play: async ({ canvasElement }) => {
    const [a, b, c, d] = ["a", "b", "c", "d"].map((name) => rectOf(canvasElement, name));
    // The first row, left to right, in three columns.
    await expect(b.top).toBe(a.top);
    await expect(b.left).toBeGreaterThan(a.right);
    await expect(c.left).toBeGreaterThan(b.right);
    // `d` in the second row, under `c`.
    await expect(d.left).toBe(c.left);
    await expect(d.top).toBeGreaterThan(c.bottom);
  },
};

/** The button the story renders with this accessible name. */
function buttonOf(canvasElement: HTMLElement, name: string): HTMLElement {
  const button = [...canvasElement.querySelectorAll<HTMLElement>("button")].find((b) => b.textContent === name);
  if (!button) throw new Error(`no button ${name}`);
  return button;
}

export const TypographyFromTokens: Story = {
  play: async ({ canvasElement }) => {
    const style = getComputedStyle(buttonOf(canvasElement, "World map"));
    // `font.size.label` 16px and `font.weight.label` 600 in default-theme.json.
    await expect(style.fontSize).toBe("16px");
    await expect(style.fontWeight).toBe("600");
    // `font.line-height.label` 1.2 and `font.family.label`.
    await expect(style.lineHeight).toBe(`${16 * 1.2}px`);
    await expect(style.fontFamily).toBe("system-ui, \"Segoe UI\", Roboto, sans-serif");
  },
};

export const ShapeAndColoursFromTokens: Story = {
  play: async ({ canvasElement }) => {
    const style = getComputedStyle(buttonOf(canvasElement, "World map"));
    // `color.button.background` #262a3b and `color.button.text` #e6e8f0.
    await expect(style.backgroundColor).toBe("rgb(38, 42, 59)");
    await expect(style.color).toBe("rgb(230, 232, 240)");
    // `size.button.border-width` 1px of `color.button.border` #626a90, and `radius.button` 10px.
    await expect(style.borderTopWidth).toBe("1px");
    await expect(style.borderTopStyle).toBe("solid");
    await expect(style.borderTopColor).toBe("rgb(98, 106, 144)");
    await expect(style.borderTopLeftRadius).toBe("10px");
    // Made for a finger: no double-tap zoom, and a press shows within
    // `motion.press-feedback` 80ms.
    await expect(style.touchAction).toBe("manipulation");
    await expect(style.transitionProperty).toBe("background-color");
    await expect(style.transitionDuration).toBe("0.08s");
  },
};

const LONG_LABEL = "A label far too long for any button of the panel to show all";

export const LongLabelEllipsis: Story = {
  // Four buttons side by side, so each cell is narrower than the label.
  args: {
    components: inRow([component("long", LONG_LABEL), component("b"), component("c"), component("d")]),
    rows: 1,
    cols: 4,
  },
  play: async ({ canvasElement }) => {
    await expect(LONG_LABEL).toHaveLength(60);
    const button = buttonOf(canvasElement, LONG_LABEL);
    const label = button.querySelector<HTMLElement>(".plinth-button__label");
    if (!label) throw new Error("no .plinth-button__label");
    const cell = button.getBoundingClientRect();
    const neighbour = buttonOf(canvasElement, "b").getBoundingClientRect();
    // The button keeps its cell, as wide as its neighbour's…
    await expect(cell.width).toBeCloseTo(neighbour.width, 1);
    // …and the label stays inside it on one line, cut with an ellipsis.
    const text = label.getBoundingClientRect();
    await expect(text.left).toBeGreaterThanOrEqual(cell.left);
    await expect(text.right).toBeLessThanOrEqual(cell.right);
    await expect(text.height).toBeCloseTo(16 * 1.2, 0);
    await expect(label.scrollWidth).toBeGreaterThan(label.clientWidth);
    // The ellipsis is painted, not laid out: it shows only on a box that
    // clips what it cuts.
    const style = getComputedStyle(label);
    await expect(style.textOverflow).toBe("ellipsis");
    await expect(style.overflowX).toBe("hidden");
  },
};

/** A page that a "Turn page" button moves to the next one, as a swipe would. */
function Turning(props: ComponentProps<typeof PageView>) {
  const [pageIndex, setPageIndex] = useState(props.pageIndex);
  return (
    <>
      <button type="button" onClick={() => setPageIndex((i) => i + 1)}>
        Turn page
      </button>
      <PageView {...props} pageIndex={pageIndex} direction="next" />
    </>
  );
}

export const PageChangeLastsTransitionMs: Story = {
  // `motion.page-transition` at its maximum, so it cannot pass for the old fixed 0.2s.
  args: { transitionMs: 400 },
  render: (args) => <Turning {...args} />,
  play: async ({ canvas, canvasElement, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Turn page" }));
    // The page leaving slides out and fades over the prop's duration: Motion
    // runs both as Web Animations, so their timing can be read.
    await waitFor(() => expect(canvasElement.getAnimations({ subtree: true }).length).toBeGreaterThan(0));
    const durations = canvasElement.getAnimations({ subtree: true }).map((a) => a.effect?.getTiming().duration);
    await expect(durations).toEqual(durations.map(() => 400));
  },
};
