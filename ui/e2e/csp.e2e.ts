import { expect, type Page, type Route, test } from "@playwright/test";

import tauriConf from "../../crates/plinth-tauri/tauri.conf.json" with { type: "json" };
import { game, installFakeBackend } from "./fakeBackend";
import { blankPoint, swipe } from "./gestures";

/** The production policy, from the same file Tauri reads; `vite preview` serves it as a header. */
const CSP = tauriConf.app.security.csp;

/** A 1×1 PNG: the image of a button, and what the remote server would answer. */
const PNG_1X1 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

type FulfillBody = NonNullable<Parameters<Route["fulfill"]>[0]>["body"];
/** Node's `Buffer`, the binary body Playwright takes: the UI's tsconfig has no Node types. */
const { Buffer } = globalThis as unknown as { Buffer: { from(data: string, encoding: "base64"): FulfillBody } };

type Violation = { directive: string; blocked: string; sample: string };

/** Records every `securitypolicyviolation` from before the page's first script runs. */
async function recordViolations(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const seen: { directive: string; blocked: string; sample: string }[] = [];
    (window as unknown as { __plinthViolations: typeof seen }).__plinthViolations = seen;
    document.addEventListener(
      "securitypolicyviolation",
      (event) => seen.push({ directive: event.effectiveDirective, blocked: event.blockedURI, sample: event.sample }),
      true,
    );
  });
}

/** The violations so far, once the reports already queued have been dispatched. */
async function violations(page: Page): Promise<Violation[]> {
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => setTimeout(done, 0))));
  return page.evaluate(() => (window as unknown as { __plinthViolations: Violation[] }).__plinthViolations);
}

test("the panel runs under the csp without violations", async ({ page }) => {
  await recordViolations(page);
  await installFakeBackend(page, {
    pages: [
      [{ ...game("fireball", "Fireball", 0, 0), imageId: "t1i0" }, game("character", "Character", 0, 1)],
      [game("bags", "Bags", 0, 0), game("spellbook", "Spellbook", 0, 1)],
    ],
    images: { t1i0: `data:image/png;base64,${PNG_1X1}` },
  });
  const response = await page.goto("/");
  expect(response?.headers()["content-security-policy"]).toBe(CSP);

  // The button image of a folder template: a `data:` URI, which `img-src` allows.
  const image = page.getByRole("button", { name: "Fireball" }).locator("img");
  await expect.poll(() => image.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(1);

  // A page change: Motion's `popLayout` writes a <style> while the old page leaves.
  await swipe(page, await blankPoint(page), -400);
  await expect(page.getByRole("button", { name: "Bags" })).toBeVisible();

  await page.getByRole("button", { name: "Lock" }).click();
  await expect(page.getByRole("button", { name: "Unlock" })).toBeVisible();

  expect(await violations(page)).toEqual([]);
});

test("a remote image is blocked by the csp", async ({ page }) => {
  let served = 0;
  // Never the network: if the policy let the image through, this would answer it.
  await page.route("https://example.com/**", (route) => {
    served += 1;
    return route.fulfill({ status: 200, contentType: "image/png", body: Buffer.from(PNG_1X1, "base64") });
  });
  await recordViolations(page);
  await installFakeBackend(page);
  await page.goto("/");
  await expect(page.getByRole("button", { name: "World map" })).toBeVisible();

  const naturalWidth = await page.evaluate(
    () =>
      new Promise<number>((done) => {
        const img = document.createElement("img");
        img.onload = img.onerror = () => done(img.naturalWidth);
        img.src = "https://example.com/x.png";
        document.body.append(img);
      }),
  );

  expect(naturalWidth).toBe(0);
  expect(served).toBe(0);
  expect(await violations(page)).toEqual([
    expect.objectContaining({ directive: "img-src", blocked: "https://example.com/x.png" }),
  ]);
});
