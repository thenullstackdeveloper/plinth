import { expect, test } from "vitest";

import { createDragGuard } from "./dragGuard";

function manualSchedule() {
  const pending: (() => void)[] = [];
  return {
    schedule: (fn: () => void) => {
      pending.push(fn);
    },
    flush: () => pending.splice(0).forEach((fn) => fn()),
  };
}

test("a click without a drag is not ignored", () => {
  const { schedule } = manualSchedule();
  expect(createDragGuard(schedule).shouldIgnoreClick()).toBe(false);
});

test("the click that ends a drag is ignored", () => {
  const { schedule } = manualSchedule();
  const guard = createDragGuard(schedule);
  guard.dragStarted();
  guard.dragEnded();
  expect(guard.shouldIgnoreClick()).toBe(true);
});

test("clicks are accepted again once the drag is over", () => {
  const { schedule, flush } = manualSchedule();
  const guard = createDragGuard(schedule);
  guard.dragStarted();
  guard.dragEnded();
  flush();
  expect(guard.shouldIgnoreClick()).toBe(false);
});
