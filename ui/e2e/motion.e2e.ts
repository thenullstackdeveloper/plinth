import { expect, type Locator, type Page, test } from "@playwright/test";

import type { Effects } from "../src/adapters/panelApi";
import defaultTheme from "../src/testing/fixtures/default-theme.json" with { type: "json" };
import { installFakeBackend } from "./fakeBackend";
import { type Frame, pageChange, readFrames, startFrames } from "./frames";
import { blankPoint, box, swipe } from "./gestures";

/** `pageTransitionMs` the fake backend sends: `motion.page-transition` of the default theme. */
const TRANSITION_MS = 200;
/** Two frames at 60 Hz, in whole milliseconds. */
const MARGIN_MS = 34;
const SWIPE_LEFT = -400;
const SWIPE_RIGHT = 400;
/** How far the elastic drag lets the surface go at an edge (`dragElastic` 0.2 of 400 px), plus 2 px. */
const EDGE_REACH_PX = 82;
/** From this long after the `pointerup` on, the refused surface is back at rest. */
const BACK_BY_MS = 300;
/** How long the recording runs after a refused swipe. */
const RECORD_MS = 700;

test("a page change lasts motion.page-transition", async ({ page }) => {
  await installFakeBackend(page);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

  const from = await blankPoint(page);
  await startFrames(page);
  await swipe(page, from, SWIPE_LEFT);
  await expect(page.getByRole("button", { name: "Bags" })).toBeVisible();
  await page.waitForTimeout(300);

  const { upAt, frames } = await readFrames(page);
  // From the first frame the slides move to the first one the new page is
  // still: the backend call and React's render before it are not the animation.
  const { start, settled } = pageChange(frames, upAt);
  expect(settled - start).toBeLessThanOrEqual(TRANSITION_MS + MARGIN_MS);
});

test("while the page changes, the leaving page never covers the sidebar", async ({ page }) => {
  await installFakeBackend(page, { sidebarSide: "left" });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

  // The sidebar's bottom-left corner, the farthest point from the page.
  const aside = await box(page.getByRole("complementary", { name: "Sidebar" }));
  const probe = { x: aside.x + 4, y: aside.y + aside.height - 4 };
  const from = await blankPoint(page);
  await startFrames(page, probe);
  await swipe(page, from, SWIPE_LEFT);
  await expect(page.getByRole("button", { name: "Bags" })).toBeVisible();
  await page.waitForTimeout(300);

  const { frames } = await readFrames(page);
  expect(frames.map((f) => f.probeHit).filter((hit) => hit !== "sidebar")).toEqual([]);
  // Both pages on screen at once: the test did see the change of page.
  expect(frames.some((f) => f.slides.length === 2)).toBe(true);
});

/**
 * Swipes `dx` from a blank point, against an edge the backend refuses, and
 * returns the frames from the `pointerup` on, after checking there are enough
 * from `BACK_BY_MS` on that the at-rest condition cannot pass without data.
 */
async function refusedSwipe(page: Page, dx: number) {
  const from = await blankPoint(page);
  await startFrames(page);
  await swipe(page, from, dx);
  await page.waitForTimeout(RECORD_MS);
  const { upAt, frames } = await readFrames(page);
  const after = frames.filter((f) => f.t >= upAt);
  const late = after.filter((f) => f.t >= upAt + BACK_BY_MS);
  expect(late.length).toBeGreaterThanOrEqual(10);
  return { after, late };
}

const surface = (f: Frame) => f.surfaces[0];

// Both conditions are soft, so a failure reports each one that does not hold.

test("a swipe refused at the first page springs back without going further", async ({ page }) => {
  await installFakeBackend(page);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

  const { after, late } = await refusedSwipe(page, SWIPE_RIGHT);
  // No further than where the finger let go: no momentum past the elastic reach.
  expect.soft(Math.max(...after.map(surface))).toBeLessThanOrEqual(EDGE_REACH_PX);
  // Back at rest, and staying there, by BACK_BY_MS after the pointerup.
  expect.soft(Math.max(...late.map((f) => Math.abs(surface(f))))).toBeLessThanOrEqual(0.5);
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();
});

test("a swipe refused at the last page springs back without going further", async ({ page }) => {
  await installFakeBackend(page);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();
  await swipe(page, await blankPoint(page), SWIPE_LEFT);
  await expect(page.getByRole("button", { name: "Bags" })).toBeVisible();
  // Let the change of page end before the next swipe grabs the new surface.
  await page.waitForTimeout(500);
  await swipe(page, await blankPoint(page), SWIPE_LEFT);
  await expect(page.getByRole("button", { name: "Quest log" })).toBeVisible();
  await page.waitForTimeout(500);

  const { after, late } = await refusedSwipe(page, SWIPE_LEFT);
  expect.soft(Math.min(...after.map(surface))).toBeGreaterThanOrEqual(-EDGE_REACH_PX);
  expect.soft(Math.max(...late.map((f) => Math.abs(surface(f))))).toBeLessThanOrEqual(0.5);
  await expect(page.getByRole("button", { name: "Quest log" })).toBeVisible();
});

// --- The effects of the catalogue ---

/**
 * Opens the panel under `effects` and swipes from page 1 to page 2,
 * recording every frame; returns them once page 2 is shown and its change
 * has had time to end.
 */
async function changePage(page: Page, effects: Partial<Effects>) {
  await installFakeBackend(page, { effects });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

  const from = await blankPoint(page);
  await startFrames(page);
  await swipe(page, from, SWIPE_LEFT);
  await expect(page.getByRole("button", { name: "Bags" })).toBeVisible();
  await page.waitForTimeout(300);
  return readFrames(page);
}

/** Whether frame `f` shows the button labelled `label` on a page. */
const shows = (f: Frame, label: string) => f.labels.includes(label);

test("fade does not move the page", async ({ page }) => {
  const { upAt, frames } = await changePage(page, { pageTransition: "fade" });
  const after = frames.filter((f) => f.t >= upAt);

  // The test did see the change: both pages on screen at once…
  expect(after.some((f) => shows(f, "World map") && shows(f, "Bags"))).toBe(true);
  // …neither of them ever off the page's edge…
  expect(frames.flatMap((f) => f.slides.map((s) => s.dx)).filter((dx) => Math.abs(dx) > 0.5)).toEqual([]);
  // …while the opacity changes: a page half seen on some frame.
  expect(after.some((f) => f.slides.some((s) => s.opacity > 0 && s.opacity < 1))).toBe(true);
});

test("none changes page with no frame in between", async ({ page }) => {
  const { upAt, frames } = await changePage(page, { pageTransition: "none" });
  const after = frames.filter((f) => f.t >= upAt);

  // The first frame with page 2 has it alone, in place and opaque…
  const first = after.find((f) => shows(f, "Bags"));
  expect(first, "a frame shows page 2").toBeDefined();
  expect(first?.slides).toHaveLength(1);
  expect(Math.abs(first?.slides[0].dx ?? Number.NaN)).toBeLessThanOrEqual(0.5);
  expect(first?.slides[0].opacity).toBe(1);
  // …and no frame ever shows both pages.
  expect(frames.filter((f) => shows(f, "World map") && shows(f, "Bags"))).toEqual([]);
  // Page 1 was on screen until then: the recording covers the change.
  expect(frames.some((f) => shows(f, "World map"))).toBe(true);
});

/** The colour Chromium paints behind a button, and its computed `transform`. */
function looksOf(locator: Locator): Promise<{ background: string; transform: string }> {
  return locator.evaluate((el) => {
    const style = getComputedStyle(el);
    return { background: style.backgroundColor, transform: style.transform };
  });
}

/** The horizontal scale of the button's `transform` matrix: 1 without one. */
async function scaleOf(locator: Locator): Promise<number> {
  const { transform } = await looksOf(locator);
  if (transform === "none") return 1;
  const match = /^matrix\(([^,]+),/.exec(transform);
  if (!match) throw new Error(`not a 2D matrix: ${transform}`);
  return Number(match[1]);
}

/** Presses the mouse down on the middle of `locator`, and checks it is held (`:active`). */
async function holdDown(page: Page, locator: Locator): Promise<void> {
  const b = await box(locator);
  await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
  await page.mouse.down();
  expect(await locator.evaluate((el) => el.matches(":active"))).toBe(true);
}

/** Longer than `motion.press-feedback` (80 ms in the default theme): any feedback has run by then. */
const FEEDBACK_RUN_MS = 250;

test("sink scales the pressed button", async ({ page }) => {
  await installFakeBackend(page, { effects: { press: "sink" } });
  await page.goto("/");
  const worldMap = page.getByRole("button", { name: "World map" });
  await expect(worldMap).toBeVisible();
  const rest = await looksOf(worldMap);
  expect(rest.transform).toBe("none");

  await holdDown(page, worldMap);
  // Shrunk to 0.96 while held…
  await expect.poll(async () => Math.abs((await scaleOf(worldMap)) - 0.96)).toBeLessThanOrEqual(0.01);
  // …in its colour at rest, not `color.button.pressed-background`.
  expect((await looksOf(worldMap)).background).toBe(rest.background);
  await page.mouse.up();
  await expect.poll(() => scaleOf(worldMap)).toBe(1);
});

test("press none gives no visual feedback", async ({ page }) => {
  await installFakeBackend(page, { effects: { press: "none" } });
  await page.goto("/");
  const worldMap = page.getByRole("button", { name: "World map" });
  await expect(worldMap).toBeVisible();
  const rest = await looksOf(worldMap);

  await holdDown(page, worldMap);
  await page.waitForTimeout(FEEDBACK_RUN_MS);
  expect(await looksOf(worldMap)).toEqual(rest);
  await page.mouse.up();
});

test("quit keeps its colours with every press effect", async ({ context }) => {
  const quitBackground = hexToRgb(defaultTheme["--plinth-color-quit-background"]);
  const toggledBackground = hexToRgb(defaultTheme["--plinth-color-button-toggled-background"]);
  for (const press of ["sink", "none"] as const) {
    const page = await context.newPage();
    await installFakeBackend(page, { effects: { press } });
    await page.goto("/");
    const quit = page.getByRole("button", { name: "Quit" });
    const lock = page.getByRole("button", { name: "Lock" });
    await lock.click();
    await expect(lock).toHaveAttribute("aria-pressed", "true");

    // Quit held down keeps the fixed `color.quit.background`…
    await holdDown(page, quit);
    await page.waitForTimeout(FEEDBACK_RUN_MS);
    expect((await looksOf(quit)).background, press).toBe(quitBackground);
    await page.mouse.move(0, 0);
    await page.mouse.up();

    // …and the lock, on and held down, its toggled colour.
    await holdDown(page, lock);
    await page.waitForTimeout(FEEDBACK_RUN_MS);
    expect((await looksOf(lock)).background, press).toBe(toggledBackground);
    await page.mouse.move(0, 0);
    await page.mouse.up();
    await page.close();
  }
});

/** `#rrggbb` as Chromium writes a computed colour. */
function hexToRgb(hex: string): string {
  const [r, g, b] = [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16));
  return `rgb(${r}, ${g}, ${b})`;
}
