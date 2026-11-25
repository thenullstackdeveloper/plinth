import type { Locator, Page } from "@playwright/test";

/** A swipe with the mouse: press at `from`, move `dx` pixels in steps, release. */
export async function swipe(page: Page, from: { x: number; y: number }, dx: number): Promise<void> {
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(from.x + dx, from.y, { steps: 12 });
  await page.mouse.up();
}

export async function box(locator: Locator) {
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
export async function blankPoint(page: Page) {
  const area = await box(page.getByRole("region", { name: "Page" }));
  return { x: area.x + 4, y: page.viewportSize()!.height - 4 };
}
