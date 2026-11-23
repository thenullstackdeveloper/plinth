import { AnimatePresence, motion } from "motion/react";
import { useRef } from "react";

import type { ComponentDto, NavDirection } from "../adapters/panelApi";
import { createDragGuard } from "../dragGuard";
import { usePanelStore } from "../store/panelStore";
import { handleSwipeEnd } from "../swipe";

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
export function PageView(props: { components: ComponentDto[]; pageIndex: number; direction: NavDirection | null }) {
  const press = usePanelStore((s) => s.press);
  const navigate = usePanelStore((s) => s.navigate);
  const sectionRef = useRef<HTMLElement>(null);
  const guard = useRef(createDragGuard()).current;
  const dir = props.direction === "prev" ? -1 : 1;

  return (
    <section aria-label="Page" ref={sectionRef}>
      <AnimatePresence mode="wait" custom={dir} initial={false}>
        <motion.div
          key={props.pageIndex}
          custom={dir}
          variants={pageVariants}
          initial="enter"
          animate="center"
          exit="exit"
          transition={{ duration: 0.2 }}
        >
          <motion.div
            drag="x"
            dragConstraints={{ left: 0, right: 0 }}
            dragElastic={0.2}
            onDragStart={() => guard.dragStarted()}
            onDragEnd={(_, info) => {
              guard.dragEnded();
              handleSwipeEnd(info, sectionRef.current?.getBoundingClientRect().width ?? 0, (d) => void navigate(d));
            }}
          >
            {props.components.map((component) => (
              <button
                key={component.id}
                type="button"
                onClick={() => {
                  if (!guard.shouldIgnoreClick()) void press(component.id);
                }}
              >
                {component.label ?? component.id}
              </button>
            ))}
          </motion.div>
        </motion.div>
      </AnimatePresence>
    </section>
  );
}
