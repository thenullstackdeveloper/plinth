import type { Page } from "@playwright/test";

/** What the panel looked like on one animation frame. */
export type Frame = {
  /** `performance.now()` at the frame. */
  t: number;
  /** Each `.page-slide`: its `x` minus the `.page`'s, in px, and its opacity. */
  slides: { dx: number; opacity: number }[];
  /** Each `.page-surface`: its `x` minus the `.page`'s, in px. */
  surfaces: number[];
  /**
   * The labels shown on the buttons of every `.page-slide`, in the page's
   * order: which pages are on screen, the leaving one and the new one during
   * a change.
   */
  labels: string[];
  /** What is under the probe point: a page slide, the sidebar, anything else; `null` without a probe. */
  probeHit: "sidebar" | "page" | "other" | null;
};

type Recording = { upAt: number | null; frames: Frame[] };

/** Where the page keeps the recording between `startFrames` and `readFrames`. */
const KEY = "__plinthFrames";

/**
 * Starts recording a `Frame` on every `requestAnimationFrame`, and the time of
 * the first `pointerup` (heard on `window`, in the capture phase, so before
 * any handler of the panel).
 */
export async function startFrames(page: Page, probe?: { x: number; y: number }): Promise<void> {
  await page.evaluate(
    ({ key, probe }) => {
      const recording: Recording = { upAt: null, frames: [] };
      (window as unknown as Record<string, Recording>)[key] = recording;
      window.addEventListener(
        "pointerup",
        () => {
          recording.upAt ??= performance.now();
        },
        { capture: true },
      );
      const region = document.querySelector(".page");
      if (!region) throw new Error("no .page to measure against");
      const offset = (el: Element) => el.getBoundingClientRect().x - region.getBoundingClientRect().x;
      const hitAt = (point: { x: number; y: number }): Frame["probeHit"] => {
        const el = document.elementFromPoint(point.x, point.y);
        if (el?.closest(".page-slide")) return "page";
        if (el?.closest(".sidebar")) return "sidebar";
        return "other";
      };
      const sample = () => {
        recording.frames.push({
          t: performance.now(),
          slides: [...document.querySelectorAll(".page-slide")].map((el) => ({
            dx: offset(el),
            opacity: Number(getComputedStyle(el).opacity),
          })),
          surfaces: [...document.querySelectorAll(".page-surface")].map(offset),
          labels: [...document.querySelectorAll(".page-slide .plinth-button__label")].map((el) => el.textContent ?? ""),
          probeHit: probe ? hitAt(probe) : null,
        });
        requestAnimationFrame(sample);
      };
      requestAnimationFrame(sample);
    },
    { key: KEY, probe },
  );
}

/** The frames recorded since `startFrames`, and the time of the first `pointerup`. */
export async function readFrames(page: Page): Promise<{ upAt: number; frames: Frame[] }> {
  const recording = await page.evaluate((key) => (window as unknown as Record<string, Recording>)[key], KEY);
  if (!recording) throw new Error("startFrames was not called");
  if (recording.upAt === null) throw new Error("no pointerup since startFrames");
  return { upAt: recording.upAt, frames: recording.frames };
}

/** Within half a pixel of the page's left edge. */
const atRest = (dx: number) => Math.abs(dx) <= 0.5;

/**
 * When a page change starts and settles, in `performance.now()` milliseconds,
 * from the `.page-slide`s alone. It starts on the first frame from `upAt` on
 * with two slides, or one moved off the page's edge; it settles on the first
 * frame from which every frame shows one slide, at the edge and fully opaque
 * (`Infinity` if the recording ends before that).
 */
export function pageChange(frames: Frame[], upAt: number): { start: number; settled: number } {
  const startIndex = frames.findIndex(
    (f) => f.t >= upAt && (f.slides.length >= 2 || f.slides.some((s) => !atRest(s.dx))),
  );
  if (startIndex === -1) throw new Error("no page change after the pointerup");
  const still = (f: Frame) => f.slides.length === 1 && atRest(f.slides[0].dx) && f.slides[0].opacity === 1;
  let settledIndex = frames.length;
  while (settledIndex > startIndex && still(frames[settledIndex - 1])) settledIndex--;
  const settled = settledIndex < frames.length ? frames[settledIndex].t : Number.POSITIVE_INFINITY;
  return { start: frames[startIndex].t, settled };
}
