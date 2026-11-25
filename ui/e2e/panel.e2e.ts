import { expect, test, type Locator, type Page } from "@playwright/test";

import defaultTheme from "../src/testing/fixtures/default-theme.json" with { type: "json" };
import maxTheme from "../src/testing/fixtures/max-theme.json" with { type: "json" };
import sliceTheme from "../src/testing/fixtures/slice-theme.json" with { type: "json" };
import { app, calls, game, installFakeBackend } from "./fakeBackend";

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
 * A point of the window with no button on it: in the page's bottom-left
 * corner, inside the surface's padding (the `1fr` grid fills the rest with
 * buttons). Taken from the window, not from the page's box, so a page that
 * does not fill the window cannot hide the defect.
 */
async function blankPoint(page: Page) {
  const area = await box(page.getByRole("region", { name: "Page" }));
  return { x: area.x + 4, y: page.viewportSize()!.height - 4 };
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
  const hit = await page.evaluate(({ x, y }) => {
    const el = document.elementFromPoint(x, y);
    return { surface: el?.closest(".page-surface") !== null, button: el?.closest("button") !== null };
  }, blank);
  // On the surface, and on no button: a swipe from there starts on the surface itself.
  expect(hit).toEqual({ surface: true, button: false });
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

  const viewport = page.viewportSize()!;

  // From the sidebar's bottom padding, below the last button, up and across
  // all of them.
  await page.mouse.move(aside.x + aside.width / 2, viewport.height - 4);
  await page.mouse.down();
  await page.mouse.move(aside.x + 2, aside.y + 2, { steps: 10 });
  await page.mouse.up();

  // The buttons stretch down to the sidebar's padding (`space.sidebar.padding` 8px).
  expect(quit.y + quit.height).toBeLessThanOrEqual(viewport.height - 8);
  expect(await page.evaluate(() => window.getSelection()?.toString() ?? "")).toBe("");
});

/** The colour Chromium paints behind a button. */
function backgroundOf(locator: Locator): Promise<string> {
  return locator.evaluate((el) => getComputedStyle(el).backgroundColor);
}

test("without a template theme the buttons take the default from tokens.json", async ({ page }) => {
  await page.goto("/");
  const worldMap = page.getByRole("button", { name: "World map" });
  await expect(worldMap).toBeVisible();
  // #262a3b, `color.button.background` in design/tokens.json.
  expect(await backgroundOf(worldMap)).toBe("rgb(38, 42, 59)");
  expect(await backgroundOf(page.getByRole("button", { name: "Lock" }))).toBe("rgb(38, 42, 59)");
});

test("a template theme colours the page buttons", async ({ page }) => {
  await installFakeBackend(page, { theme: sliceTheme });
  await page.goto("/");
  const worldMap = page.getByRole("button", { name: "World map" });
  await expect(worldMap).toBeVisible();
  // #1e1e2e, the override in slice-theme.json.
  expect(await backgroundOf(worldMap)).toBe("rgb(30, 30, 46)");
});

test("buttons sit in the cell their row and col give", async ({ page }) => {
  // A 2×3 grid: `a` and `b` in the first row, `c` in the last cell.
  await installFakeBackend(page, {
    pages: [[game("a", "A", 0, 0), game("b", "B", 0, 1), game("c", "C", 1, 2)]],
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "A", exact: true })).toBeVisible();

  const area = await box(page.getByRole("region", { name: "Page" }));
  const [a, b, c] = await Promise.all(["A", "B", "C"].map((name) => box(page.getByRole("button", { name, exact: true }))));
  // `space.page.padding` 12 and `space.page.gap` 8 in default-theme.json.
  const padding = 12;
  const gap = 8;
  expect(a.x).toBeCloseTo(area.x + padding, 1);
  expect(a.y).toBeCloseTo(area.y + padding, 1);
  expect(b.y).toBeCloseTo(a.y, 1);
  expect(b.x).toBeCloseTo(a.x + a.width + gap, 1);
  expect(c.y).toBeCloseTo(a.y + a.height + gap, 1);
  expect(c.x).toBeCloseTo(b.x + b.width + gap, 1);
  // `1fr` rows and columns: every cell the same size, and the grid fills the
  // page up to its padding.
  expect(c.width).toBeCloseTo(a.width, 1);
  expect(c.height).toBeCloseTo(a.height, 1);
  expect(c.x + c.width).toBeCloseTo(area.x + area.width - padding, 1);
  expect(c.y + c.height).toBeCloseTo(area.y + area.height - padding, 1);
});

for (const [name, theme] of [
  ["default", defaultTheme],
  ["max", maxTheme],
] as const) {
  test(`with the ${name} theme the sidebar is as wide as its token and the page takes the rest`, async ({ page }) => {
    await installFakeBackend(page, { theme });
    await page.goto("/");
    await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

    const aside = await box(page.getByRole("complementary", { name: "Sidebar" }));
    const area = await box(page.getByRole("region", { name: "Page" }));
    expect(aside.width).toBe(Number.parseInt(theme["--plinth-size-sidebar-width"], 10));
    expect(aside.width + area.width).toBe(page.viewportSize()!.width);
  });
}

test("with the max theme Quit stays inside the window", async ({ page }) => {
  // The most sidebar buttons the max theme fits (`check_fit`: 4·44 + 3·16 +
  // 2·16 = 256 ≤ 300), with Quit last.
  await installFakeBackend(page, {
    theme: maxTheme,
    sidebar: [
      app("lock", "Lock", "toggle_lock", 0),
      app("switch_side", "Switch side", "switch_sidebar", 1),
      app("next", "Next", "next_page", 2),
      app("quit", "Quit", "quit", 3),
    ],
  });
  await page.goto("/");
  const quit = page.getByRole("button", { name: "Quit" });
  await expect(quit).toBeVisible();

  const b = await box(quit);
  const viewport = page.viewportSize()!;
  expect(b.x).toBeGreaterThanOrEqual(0);
  expect(b.y).toBeGreaterThanOrEqual(0);
  expect(b.x + b.width).toBeLessThanOrEqual(viewport.width);
  expect(b.y + b.height).toBeLessThanOrEqual(viewport.height);
  // Nothing covers it: a tap on its centre lands on Quit.
  const hit = await page.evaluate(
    ({ x, y }) => document.elementFromPoint(x, y)?.closest("button")?.textContent ?? null,
    { x: b.x + b.width / 2, y: b.y + b.height / 2 },
  );
  expect(hit).toBe("Quit");
});

test("a hostile theme value never reaches the panel and fetches nothing", async ({ page }) => {
  // Neither can reach the page from Rust, which writes every value from
  // numbers; `themeStyle` is the last barrier if one ever did.
  const hostile = {
    ...defaultTheme,
    "--plinth-color-panel-background": "url(http://127.0.0.1:9/value)",
    "background-image": "url(http://127.0.0.1:9/name)",
  };
  await installFakeBackend(page, { theme: hostile });
  const requests: string[] = [];
  page.on("request", (request) => requests.push(request.url()));

  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();
  await page.waitForLoadState("networkidle");

  const panel = await page.locator(".panel").evaluate((el) => {
    const style = getComputedStyle(el);
    return {
      variable: style.getPropertyValue("--plinth-color-panel-background"),
      image: style.backgroundImage,
    };
  });
  expect(panel).toEqual({ variable: "", image: "none" });
  expect(requests.filter((url) => url.startsWith("http://127.0.0.1:9/"))).toEqual([]);
});

test("a button held down shows the pressed colour", async ({ page }) => {
  await page.goto("/");
  const worldMap = page.getByRole("button", { name: "World map" });
  await expect(worldMap).toBeVisible();
  const b = await box(worldMap);

  await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
  await page.mouse.down();
  // #343a55, `color.button.pressed-background`, once the press feedback has run.
  await expect.poll(() => backgroundOf(worldMap)).toBe("rgb(52, 58, 85)");
  await page.mouse.up();
  await expect.poll(() => backgroundOf(worldMap)).toBe("rgb(38, 42, 59)");
});

test("Quit keeps its colours under a template theme", async ({ page }) => {
  await installFakeBackend(page, { theme: sliceTheme });
  await page.goto("/");
  const quit = page.getByRole("button", { name: "Quit" });
  await expect(quit).toBeVisible();
  // The theme reaches the other buttons (#1e1e2e)…
  expect(await backgroundOf(page.getByRole("button", { name: "Lock" }))).toBe("rgb(30, 30, 46)");
  // …but Quit keeps the fixed `color.quit.background` #b23a48 and `color.quit.text` #ffffff.
  expect(await quit.evaluate((el) => [getComputedStyle(el).backgroundColor, getComputedStyle(el).color])).toEqual([
    "rgb(178, 58, 72)",
    "rgb(255, 255, 255)",
  ]);
});
