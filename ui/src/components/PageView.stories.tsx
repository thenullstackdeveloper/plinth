import type { Meta, StoryObj } from "@storybook/react-vite";
import { invoke } from "@tauri-apps/api/core";
import { type ComponentProps, useState } from "react";
import { expect, mocked, waitFor } from "storybook/test";

import { usePanelStore } from "../store/panelStore";
import { component, hidingLabel, inRow, viewWith, withIcon, withImage } from "../testing/views";
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

/** A 4×4 PNG drawn in the story's own browser, as the backend would send it. */
function drawnPng(): string {
  const canvas = document.createElement("canvas");
  canvas.width = 4;
  canvas.height = 4;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("no 2d context");
  context.fillStyle = "#c0392b";
  context.fillRect(0, 0, 4, 4);
  return canvas.toDataURL("image/png");
}

export const ButtonImage: Story = {
  args: { components: [withImage(worldMap, "t1i0")] },
  beforeEach: () => {
    usePanelStore.setState({ images: { t1i0: drawnPng() } });
  },
  play: async ({ canvas, canvasElement }) => {
    // The image does not take the button's name: it keeps its label.
    const button = canvas.getByRole("button", { name: "World map" });
    const image = button.querySelector<HTMLImageElement>("img");
    if (!image) throw new Error("no img in the button");
    await expect(image.alt).toBe("");
    await waitFor(() => expect(image.naturalWidth).toBe(4));
    await expect(getComputedStyle(image).objectFit).toBe("cover");
    // It fills the face of the button, inside its 1px border.
    const face = button.getBoundingClientRect();
    const box = image.getBoundingClientRect();
    await expect(box.left - face.left).toBeCloseTo(1, 1);
    await expect(face.right - box.right).toBeCloseTo(1, 1);
    await expect(box.top - face.top).toBeCloseTo(1, 1);
    await expect(face.bottom - box.bottom).toBeCloseTo(1, 1);
    // An id the table does not have paints nothing.
    usePanelStore.setState({ images: {} });
    await waitFor(() => expect(canvasElement.querySelector("img")).toBeNull());
  },
};

/** The first element of `selector` inside `parent`. */
function partOf<T extends HTMLElement>(parent: HTMLElement, selector: string): T {
  const el = parent.querySelector<T>(selector);
  if (!el) throw new Error(`no ${selector}`);
  return el;
}

const centreY = (r: DOMRect) => r.top + r.height / 2;
const centreX = (r: DOMRect) => r.left + r.width / 2;

export const IconAndLabel: Story = {
  args: { components: [withIcon(worldMap, "t1i0")] },
  beforeEach: () => {
    usePanelStore.setState({ images: { t1i0: drawnPng() } });
  },
  play: async ({ canvas }) => {
    const button = canvas.getByRole("button", { name: "World map" });
    const icon = partOf<HTMLImageElement>(button, "img.plinth-button__icon");
    await expect(icon.alt).toBe("");
    await waitFor(() => expect(icon.naturalWidth).toBe(4));
    const label = partOf(button, ".plinth-button__label");
    await expect(label).toBeVisible();
    // The icon to the left of the label, on the same line, both inside the button.
    const [face, iconBox, text] = [button, icon, label].map((el) => el.getBoundingClientRect());
    await expect(iconBox.width).toBeGreaterThan(0);
    await expect(iconBox.right).toBeLessThanOrEqual(text.left);
    await expect(centreY(iconBox)).toBeCloseTo(centreY(text), 0);
    await expect(iconBox.left).toBeGreaterThanOrEqual(face.left);
    await expect(text.right).toBeLessThanOrEqual(face.right);
  },
};

export const IconOnly: Story = {
  args: { components: [hidingLabel(withIcon(worldMap, "t1i0"))] },
  beforeEach: () => {
    usePanelStore.setState({ images: { t1i0: drawnPng() } });
  },
  play: async ({ canvas }) => {
    // The label is no longer seen, but it is still the button's name.
    const button = canvas.getByRole("button", { name: "World map" });
    const hidden = partOf(button, ".plinth-visually-hidden");
    await expect(hidden.textContent).toBe("World map");
    await expect(getComputedStyle(hidden).clip).toBe("rect(0px, 0px, 0px, 0px)");
    await expect(button.querySelector(".plinth-button__label")).toBeNull();
    // The icon alone, in the middle of the button.
    const icon = partOf<HTMLImageElement>(button, "img.plinth-button__icon");
    await waitFor(() => expect(icon.naturalWidth).toBe(4));
    const [face, box] = [button, icon].map((el) => el.getBoundingClientRect());
    await expect(centreX(box)).toBeCloseTo(centreX(face), 0);
    await expect(centreY(box)).toBeCloseTo(centreY(face), 0);
  },
};

export const ImageWithLabel: Story = {
  // An icon as well is refused by Rust; were one to come, the image wins.
  args: { components: [withIcon(withImage(worldMap, "t1i0"), "t1i0")] },
  beforeEach: () => {
    usePanelStore.setState({ images: { t1i0: drawnPng() } });
  },
  play: async ({ canvas }) => {
    const button = canvas.getByRole("button", { name: "World map" });
    const image = partOf<HTMLImageElement>(button, "img.plinth-button__image");
    await waitFor(() => expect(image.naturalWidth).toBe(4));
    await expect(button.querySelectorAll("img")).toHaveLength(1);
    // The label reads on a strip at the bottom of the face, over the image.
    const label = partOf(button, ".plinth-button__label");
    const [face, strip] = [image, label].map((el) => el.getBoundingClientRect());
    await expect(strip.left).toBeCloseTo(face.left, 1);
    await expect(strip.right).toBeCloseTo(face.right, 1);
    await expect(strip.bottom).toBeCloseTo(face.bottom, 1);
    await expect(strip.height).toBeLessThan(face.height);
    const top = document.elementFromPoint(centreX(strip), centreY(strip));
    await expect(top === label || label.contains(top)).toBe(true);
    await expect(getComputedStyle(label).backgroundColor).not.toBe("rgba(0, 0, 0, 0)");
  },
};
