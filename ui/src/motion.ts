/** Motion's transition for the change of page, from `motion.page-transition` in milliseconds. */
export function pageTransition(ms: number): { duration: number } {
  return { duration: ms / 1000 };
}

/**
 * The spring that brings the surface back to its place when a swipe does not
 * change page (an edge refused, a swipe too short). Close to critical damping
 * (2·√1000 ≈ 63), so it comes back fast without overshooting, and fixed: it
 * does not follow `motion.page-transition`, because it is not a change of page.
 */
export const snapBack = { bounceStiffness: 1000, bounceDamping: 63 };
