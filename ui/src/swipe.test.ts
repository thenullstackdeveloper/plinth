import { expect, test, vi } from "vitest";

import { handleSwipeEnd, swipeDirection } from "./swipe";

const WIDTH = 900;

test("a long drag to the left goes to the next page", () => {
  expect(swipeDirection(-300, 0, WIDTH)).toBe("next");
});

test("a long drag to the right goes to the previous page", () => {
  expect(swipeDirection(300, 0, WIDTH)).toBe("prev");
});

test("a short slow drag does nothing", () => {
  expect(swipeDirection(-100, 0, WIDTH)).toBeNull();
});

test("a short fast flick to the left goes to the next page", () => {
  expect(swipeDirection(-50, -800, WIDTH)).toBe("next");
});

test("the offset wins over an opposite velocity", () => {
  expect(swipeDirection(300, -800, WIDTH)).toBe("prev");
});

test("without a measured width only the velocity counts", () => {
  expect(swipeDirection(-300, 0, 0)).toBeNull();
});

test("handleSwipeEnd navigates once on a long drag", () => {
  const navigate = vi.fn();
  handleSwipeEnd({ offset: { x: -300 }, velocity: { x: 0 } }, WIDTH, navigate);
  expect(navigate).toHaveBeenCalledTimes(1);
  expect(navigate).toHaveBeenCalledWith("next");
});

test("handleSwipeEnd does nothing on a short slow drag", () => {
  const navigate = vi.fn();
  handleSwipeEnd({ offset: { x: -100 }, velocity: { x: 0 } }, WIDTH, navigate);
  expect(navigate).not.toHaveBeenCalled();
});
