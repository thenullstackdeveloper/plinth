import { expect, test } from "vitest";

import { pageTransition } from "./motion";

test("pageTransition turns the token's milliseconds into Motion's seconds", () => {
  expect(pageTransition(200)).toEqual({ duration: 0.2 });
});

test("pageTransition of 0ms changes page at once", () => {
  expect(pageTransition(0)).toEqual({ duration: 0 });
});
