import { expect, test, type Locator, type Page } from "@playwright/test";

import { calls, installFakeBackend } from "./fakeBackend";

/** A swipe with the mouse: press at `from`, move `dx` pixels in steps, release. */
async function swipe(page: Page, from: { x: number; y: number }, dx: number): Promise<void> {
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(from.x + dx, from.y, { steps: 12 });
  await page.mouse.up();
}

async function box(locator: Locator) {
  const b = await locator.boundingBox();
  if (!b) throw new Error("element has no box");
  return b;
}

/**
 * A point of the window with no button on it: in the page's column, near the
 * bottom of the window. Taken from the window, not from the page's box, so a
 * page that does not fill the window cannot hide the defect.
 */
async function blankPoint(page: Page) {
  const area = await box(page.getByRole("region", { name: "Page" }));
  return { x: area.x + 20, y: page.viewportSize()!.height - 20 };
}

const SWIPE_LEFT = -400;

test.beforeEach(async ({ page }) => {
  await installFakeBackend(page);
});

test("the draggable surface covers the whole page, which fills the window's height", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

  const area = await box(page.getByRole("region", { name: "Page" }));
  const surface = await box(page.locator(".page-surface"));
  const viewport = page.viewportSize()!;
  expect(area.height).toBe(viewport.height);
  expect(surface).toEqual(area);

  const blank = await blankPoint(page);
  const hitsSurface = await page.evaluate(
    ({ x, y }) => document.elementFromPoint(x, y)?.closest(".page-surface") !== null,
    blank,
  );
  expect(hitsSurface).toBe(true);
});

test("a swipe from a blank point changes page and selects no text", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

  await swipe(page, await blankPoint(page), SWIPE_LEFT);

  await expect(page.getByRole("button", { name: "Bags" })).toBeVisible();
  expect(await page.evaluate(() => window.getSelection()?.toString() ?? "")).toBe("");
});

test("a swipe that starts on a button changes page without pressing it", async ({ page }) => {
  await page.goto("/");
  const worldMap = await box(page.getByRole("button", { name: "World map" }));

  // A short, fast flick released over the same button: the browser would
  // click it, so only the drag guard keeps it from being pressed.
  const y = worldMap.y + worldMap.height / 2;
  await page.mouse.move(worldMap.x + worldMap.width - 4, y);
  await page.mouse.down();
  await page.mouse.move(worldMap.x + 4, y, { steps: 3 });
  await page.mouse.up();

  await expect(page.getByRole("button", { name: "Bags" })).toBeVisible();
  const presses = (await calls(page)).filter((c) => c.cmd === "press");
  expect(presses).toEqual([]);
});

test("with the lock on, a swipe does not change page", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Lock" }).click();
  await expect(page.getByRole("button", { name: "Unlock" })).toBeVisible();

  // Mid-drag, the page does not follow the pointer.
  const from = await blankPoint(page);
  const before = await box(page.locator(".page-surface"));
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(from.x - 200, from.y, { steps: 8 });
  expect((await box(page.locator(".page-surface"))).x).toBe(before.x);
  await page.mouse.up();

  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();
  expect((await calls(page)).filter((c) => c.cmd === "navigate")).toEqual([]);
});

for (const side of ["left", "right"] as const) {
  test(`the sidebar sits on the ${side} of the page, at the same height`, async ({ page }) => {
    await installFakeBackend(page, { sidebarSide: side });
    await page.goto("/");
    await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

    const aside = await box(page.getByRole("complementary", { name: "Sidebar" }));
    const area = await box(page.getByRole("region", { name: "Page" }));
    expect(aside.y).toBe(area.y);
    // Between them they fill the window's width: the page takes what the sidebar leaves.
    expect(aside.width + area.width).toBe(page.viewportSize()!.width);
    if (side === "left") expect(aside.x + aside.width).toBeLessThanOrEqual(area.x);
    else expect(area.x + area.width).toBeLessThanOrEqual(aside.x);
  });
}

test("an error from the backend shows inside the window", async ({ page }) => {
  await installFakeBackend(page, {
    failPress: { componentId: "world_map", error: "action: input layer error" },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "World map" }).click();

  const alert = page.getByRole("alert");
  await expect(alert).toHaveText("action: input layer error");
  const b = await box(alert);
  expect(b.y + b.height).toBeLessThanOrEqual(page.viewportSize()!.height);
});

test("a drag across the sidebar selects no text", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();
  const aside = await box(page.getByRole("complementary", { name: "Sidebar" }));
  const quit = await box(page.getByRole("button", { name: "Quit" }));

  // From below the last sidebar button, up and across all of them.
  await page.mouse.move(aside.x + aside.width / 2, page.viewportSize()!.height - 10);
  await page.mouse.down();
  await page.mouse.move(aside.x + 2, aside.y + 2, { steps: 10 });
  await page.mouse.up();

  expect(quit.y + quit.height).toBeLessThan(page.viewportSize()!.height - 10);
  expect(await page.evaluate(() => window.getSelection()?.toString() ?? "")).toBe("");
});
