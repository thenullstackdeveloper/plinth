import type { NavDirection } from "./adapters/panelApi";

/** Fraction of the page width a drag must cover to change page. */
const DISTANCE_THRESHOLD = 0.25;
/** Flick speed, in px/s, that changes page even on a short drag. */
const VELOCITY_THRESHOLD = 500;

/**
 * Decides the page change at the end of a drag. The distance wins over the
 * velocity; dragging left goes to the next page. Without a measured width only
 * the velocity counts.
 */
export function swipeDirection(offsetX: number, velocityX: number, width: number): NavDirection | null {
  if (width > 0) {
    if (offsetX <= -width * DISTANCE_THRESHOLD) return "next";
    if (offsetX >= width * DISTANCE_THRESHOLD) return "prev";
  }
  if (velocityX <= -VELOCITY_THRESHOLD) return "next";
  if (velocityX >= VELOCITY_THRESHOLD) return "prev";
  return null;
}

/** The `onDragEnd` logic of the page, free of Motion so it can be tested. Does nothing while locked. */
export function handleSwipeEnd(
  info: { offset: { x: number }; velocity: { x: number } },
  width: number,
  locked: boolean,
  navigate: (direction: NavDirection) => void,
): void {
  if (locked) return;
  const direction = swipeDirection(info.offset.x, info.velocity.x, width);
  if (direction !== null) navigate(direction);
}
