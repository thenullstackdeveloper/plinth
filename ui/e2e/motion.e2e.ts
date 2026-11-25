import { expect, test } from "@playwright/test";

import { installFakeBackend } from "./fakeBackend";
import { pageChange, readFrames, startFrames } from "./frames";
import { blankPoint, box, swipe } from "./gestures";

/** `pageTransitionMs` the fake backend sends: `motion.page-transition` of the default theme. */
const TRANSITION_MS = 200;
/** Two frames at 60 Hz, in whole milliseconds. */
const MARGIN_MS = 34;
const SWIPE_LEFT = -400;

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
