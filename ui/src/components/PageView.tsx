import { AnimatePresence, motion } from "motion/react";
import { useRef } from "react";

import type { ComponentDto, NavDirection } from "../adapters/panelApi";
import { createDragGuard } from "../dragGuard";
import { pageTransition } from "../motion";
import { usePanelStore } from "../store/panelStore";
import { handleSwipeEnd } from "../swipe";

/** `n` grid tracks, each at least a button's minimum size, sharing the rest. */
function tracks(n: number): string {
  return `repeat(${n}, minmax(var(--plinth-size-button-min), 1fr))`;
}

const pageVariants = {
  enter: (dir: number) => ({ transform: `translateX(${dir * 100}%)`, opacity: 0 }),
  center: { transform: "translateX(0%)", opacity: 1 },
  exit: (dir: number) => ({ transform: `translateX(${-dir * 100}%)`, opacity: 0 }),
};

/**
 * One page of buttons. The outer element animates the page change with a
 * transform string; the inner one is the draggable surface, because Motion's
 * drag moves `x` and a transform string on the same element would cancel it.
 */
export function PageView(props: {
  components: ComponentDto[];
  /** The size of the page's grid. */
  rows: number;
  cols: number;
  pageIndex: number;
  direction: NavDirection | null;
  locked: boolean;
  /** How long the change of page takes, in milliseconds. */
  transitionMs: number;
}) {
  const press = usePanelStore((s) => s.press);
  const navigate = usePanelStore((s) => s.navigate);
  const sectionRef = useRef<HTMLElement>(null);
  const guard = useRef(createDragGuard()).current;
  const dir = props.direction === "prev" ? -1 : 1;

  return (
    <section className="page" aria-label="Page" ref={sectionRef}>
      {/* `popLayout`: the leaving and the entering page slide at the same
          time, so the whole change lasts `transitionMs`, not twice that. */}
      <AnimatePresence mode="popLayout" custom={dir} initial={false}>
        <motion.div
          key={props.pageIndex}
          className="page-slide"
          custom={dir}
          variants={pageVariants}
          initial="enter"
          animate="center"
          exit="exit"
          transition={pageTransition(props.transitionMs)}
        >
          <motion.div
            className="page-surface"
            style={{ gridTemplateRows: tracks(props.rows), gridTemplateColumns: tracks(props.cols) }}
            drag={props.locked ? false : "x"}
            dragConstraints={{ left: 0, right: 0 }}
            dragElastic={0.2}
            onDragStart={() => guard.dragStarted()}
            onDragEnd={(_, info) => {
              guard.dragEnded();
              handleSwipeEnd(info, sectionRef.current?.getBoundingClientRect().width ?? 0, props.locked, (d) => void navigate(d));
            }}
          >
            {props.components.map((component) => (
              <button
                key={component.id}
                type="button"
                className="plinth-button"
                data-app-action={component.appAction ?? undefined}
                style={{ gridRow: component.row + 1, gridColumn: component.col + 1 }}
                onClick={() => {
                  if (!guard.shouldIgnoreClick()) void press(component.id);
                }}
              >
                <span className="plinth-button__label">{component.label ?? component.id}</span>
              </button>
            ))}
          </motion.div>
        </motion.div>
      </AnimatePresence>
    </section>
  );
}
