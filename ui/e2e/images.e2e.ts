import { expect, type Page, test } from "@playwright/test";

import { calls, emitPanelChanged, game, installFakeBackend } from "./fakeBackend";
import { box } from "./gestures";

/**
 * A `size` × `size` image drawn by Chromium on the blank page the test starts
 * on, before the panel loads: what Rust would send as a `data:` URI. A WebP
 * at quality 1 is lossless.
 */
async function drawnImage(page: Page, size: number, type: "image/png" | "image/webp"): Promise<string> {
  return page.evaluate(
    ([side, format]) => {
      const canvas = document.createElement("canvas");
      canvas.width = side;
      canvas.height = side;
      const context = canvas.getContext("2d");
      if (!context) throw new Error("no 2d context");
      context.fillStyle = "#c0392b";
      context.fillRect(0, 0, side, side);
      return canvas.toDataURL(format, 1);
    },
    [size, type] as const,
  );
}

const drawnPng = (page: Page, size: number) => drawnImage(page, size, "image/png");

test("a button image fills the button and keeps its name", async ({ page }) => {
  const png = await drawnPng(page, 8);
  await installFakeBackend(page, {
    pages: [
      [
        { ...game("fireball", "Fireball", 0, 0), imageId: "t1i0" },
        game("character", "Character", 0, 1),
        // An SVG can carry a script: the store drops it, so it is never painted.
        { ...game("bags", "Bags", 0, 2), imageId: "t1i1" },
      ],
    ],
    images: { t1i0: png, t1i1: "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciLz4=" },
  });
  await page.goto("/");

  const button = page.getByRole("button", { name: "Fireball" });
  await expect(button).toBeVisible();
  const image = button.locator("img");
  await expect.poll(() => image.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBeGreaterThan(0);
  expect(await image.evaluate((img) => getComputedStyle(img).objectFit)).toBe("cover");

  // The face of the button: its box less `size.button.border-width` on every side.
  const border = await button.evaluate((el) => parseFloat(getComputedStyle(el).borderTopWidth));
  expect(border).toBeGreaterThan(0);
  const face = await box(button);
  const drawn = await box(image);
  expect(drawn.x).toBeCloseTo(face.x + border, 1);
  expect(drawn.y).toBeCloseTo(face.y + border, 1);
  expect(drawn.width).toBeCloseTo(face.width - 2 * border, 1);
  expect(drawn.height).toBeCloseTo(face.height - 2 * border, 1);

  // The button without an image has none, nor the one whose image is not a PNG or a WebP.
  await expect(page.getByRole("button", { name: "Character" }).locator("img")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Bags" }).locator("img")).toHaveCount(0);
});

test("the panel asks for the template images once at start and after a template change", async ({ page }) => {
  const [first, second] = [await drawnPng(page, 4), await drawnPng(page, 6)];
  await installFakeBackend(page, {
    pages: [[{ ...game("fireball", "Fireball", 0, 0), imageId: "t1i0" }]],
    images: { t1i0: first },
  });
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Fireball" }).locator("img")).toHaveAttribute("src", first);

  const atStart = (await calls(page)).map((c) => c.cmd);
  expect(atStart.filter((cmd) => cmd === "template_images")).toHaveLength(1);
  expect(atStart.indexOf("template_images")).toBeLessThan(atStart.indexOf("render_state"));

  await emitPanelChanged(page, {
    pages: [[{ ...game("frostbolt", "Frostbolt", 0, 0), imageId: "t2i0" }]],
    images: { t2i0: second },
  });
  await expect(page.getByRole("button", { name: "Frostbolt" }).locator("img")).toHaveAttribute("src", second);

  // Exactly one more, and before the view it belongs to.
  const after = (await calls(page)).map((c) => c.cmd).slice(atStart.length);
  expect(after.filter((cmd) => cmd === "template_images")).toHaveLength(1);
  expect(after.filter((cmd) => cmd === "render_state")).toHaveLength(1);
  expect(after.indexOf("template_images")).toBeLessThan(after.indexOf("render_state"));
});

test("a webp button image renders", async ({ page }) => {
  const webp = await drawnImage(page, 6, "image/webp");
  // Chromium falls back to PNG for a format it cannot write.
  expect(webp).toMatch(/^data:image\/webp;base64,/);
  await installFakeBackend(page, {
    pages: [[{ ...game("fireball", "Fireball", 0, 0), imageId: "t1i0" }]],
    images: { t1i0: webp },
  });
  await page.goto("/");

  const image = page.getByRole("button", { name: "Fireball" }).locator("img");
  await expect(image).toHaveAttribute("src", webp);
  await expect.poll(() => image.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(6);
});
