/** Motion's transition for the change of page, from `motion.page-transition` in milliseconds. */
export function pageTransition(ms: number): { duration: number } {
  return { duration: ms / 1000 };
}
