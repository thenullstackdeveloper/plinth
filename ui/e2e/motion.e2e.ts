import { expect, type Page, test } from "@playwright/test";

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
