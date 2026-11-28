import type { Effects } from "./adapters/panelApi";

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

/** A change of page of the template's `effects.page_transition` catalogue. */
export type PageTransitionKind = Effects["pageTransition"];

/** One state of a page during its change: where it is, how opaque, and, for `none`, its own transition. */
type PageState = { transform: string; opacity: number; transition?: { duration: number } };

/**
 * Motion's variants for the change of page of `kind`, for a `.page-slide`:
 * `enter` and `exit` take the direction (1 to the next page, -1 to the one
 * before). The duration is the component's (`pageTransition`), except for
 * `none`, whose states carry a transition of 0, which wins over it.
 */
export function pageVariantsFor(kind: PageTransitionKind): {
  enter: (dir: number) => PageState;
  center: PageState;
  exit: (dir: number) => PageState;
} {
  switch (kind) {
    case "slide":
      return {
        enter: (dir) => ({ transform: `translateX(${dir * 100}%)`, opacity: 0 }),
        center: { transform: "translateX(0%)", opacity: 1 },
        exit: (dir) => ({ transform: `translateX(${-dir * 100}%)`, opacity: 0 }),
      };
    case "fade":
      return {
        enter: () => ({ transform: "translateX(0%)", opacity: 0 }),
        center: { transform: "translateX(0%)", opacity: 1 },
        exit: () => ({ transform: "translateX(0%)", opacity: 0 }),
      };
    case "none": {
      const still = { transform: "translateX(0%)", opacity: 1, transition: { duration: 0 } };
      return { enter: () => still, center: still, exit: () => still };
    }
  }
}
