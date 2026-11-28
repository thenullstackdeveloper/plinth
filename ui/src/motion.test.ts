import { expect, test } from "vitest";

import { pageTransition, pageVariantsFor } from "./motion";

test("pageTransition turns the token's milliseconds into Motion's seconds", () => {
  expect(pageTransition(200)).toEqual({ duration: 0.2 });
});

test("pageTransition of 0ms changes page at once", () => {
  expect(pageTransition(0)).toEqual({ duration: 0 });
});

test("pageVariantsFor gives no movement for fade and no duration for none", () => {
  const directions = [1, -1];
  // `slide` moves the pages across, from the side the change comes from.
  const slide = pageVariantsFor("slide");
  expect(directions.map((dir) => [slide.enter(dir).transform, slide.exit(dir).transform])).toEqual([
    ["translateX(100%)", "translateX(-100%)"],
    ["translateX(-100%)", "translateX(100%)"],
  ]);
  // `fade` only fades: every state is at the page's place, whatever the direction.
  const fade = pageVariantsFor("fade");
  for (const dir of directions) {
    expect(fade.enter(dir)).toEqual({ transform: "translateX(0%)", opacity: 0 });
    expect(fade.exit(dir)).toEqual({ transform: "translateX(0%)", opacity: 0 });
  }
  expect(fade.center).toEqual({ transform: "translateX(0%)", opacity: 1 });
  // `none` is the new page at once: in place, opaque, and with no duration,
  // which a variant's own transition gives over the component's.
  const none = pageVariantsFor("none");
  const still = { transform: "translateX(0%)", opacity: 1, transition: { duration: 0 } };
  for (const dir of directions) {
    expect(none.enter(dir)).toEqual(still);
    expect(none.exit(dir)).toEqual(still);
  }
  expect(none.center).toEqual(still);
});
