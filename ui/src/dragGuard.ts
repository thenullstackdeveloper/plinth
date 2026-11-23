export type DragGuard = {
  dragStarted(): void;
  dragEnded(): void;
  shouldIgnoreClick(): boolean;
};

/**
 * Discards the click a browser fires right after a drag ends, so a swipe that
 * starts on a button does not press it. The mark is cleared on the next tick,
 * after that click has been dispatched.
 */
export function createDragGuard(schedule: (fn: () => void) => void = (fn) => setTimeout(fn, 0)): DragGuard {
  let dragging = false;
  return {
    dragStarted() {
      dragging = true;
    },
    dragEnded() {
      schedule(() => {
        dragging = false;
      });
    },
    shouldIgnoreClick: () => dragging,
  };
}
