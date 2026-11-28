import { AnimatePresence, motion } from "motion/react";
import { type CSSProperties, useRef } from "react";

import type { ComponentDto, NavDirection, TemplateImages } from "../adapters/panelApi";
import { createDragGuard } from "../dragGuard";
import { safeImageSrc } from "../image";
import { pageTransition, snapBack } from "../motion";
import { usePanelStore } from "../store/panelStore";
import { handleSwipeEnd } from "../swipe";
import { themeStyle } from "../theme";

/** `n` grid tracks, each at least a button's minimum size, sharing the rest. */
function tracks(n: number): string {
  return `repeat(${n}, minmax(var(--plinth-size-button-min), 1fr))`;
}

/** The grid lines a button covers: from one more than its `row` and `col`, over its spans (1 when absent). */
function cellsOf(component: ComponentDto): CSSProperties {
  return {
    gridRow: `${component.row + 1} / span ${component.rowSpan ?? 1}`,
    gridColumn: `${component.col + 1} / span ${component.colSpan ?? 1}`,
  };
}

const pageVariants = {
  enter: (dir: number) => ({ transform: `translateX(${dir * 100}%)`, opacity: 0 }),
  center: { transform: "translateX(0%)", opacity: 1 },
  exit: (dir: number) => ({ transform: `translateX(${-dir * 100}%)`, opacity: 0 }),
};

/** The URI of image `id`, only if the table has it: the store kept only the URIs `safeImageSrc` lets through. */
function imageOf(images: TemplateImages, id: string | undefined): string | null {
  return id !== undefined && Object.hasOwn(images, id) ? images[id] : null;
}

/**
 * The `background-image` of a page, built only from a URI that passes
 * `safeImageSrc`, never from the template's text; nothing without one.
 */
function backgroundOf(uri: string | null): CSSProperties {
  const safe = safeImageSrc(uri);
  return safe === null ? {} : { backgroundImage: `url("${safe}")` };
}

/**
 * A panel button, on a page or in the sidebar, and what it shows: an image
 * filling its face (only where `allowImage`, the pages) or an icon next to
 * its label, and the label, seen or hidden. The images are decorative, so
 * `label` is always the button's name, even when it is hidden from view.
 */
export function PanelButton(props: {
  component: ComponentDto;
  /** The button's name, shown unless the component hides it. */
  label: string;
  allowImage: boolean;
  pressed?: boolean;
  style?: CSSProperties;
  onClick: () => void;
}) {
  const images = usePanelStore((s) => s.images);
  const { component } = props;
  const image = props.allowImage ? imageOf(images, component.imageId) : null;
  // An icon or an image, never both: Rust refuses a template with both.
  const icon = image === null ? imageOf(images, component.iconId) : null;
  const className = ["plinth-button", image !== null && "plinth-button--image", icon !== null && "plinth-button--icon"]
    .filter(Boolean)
    .join(" ");
  return (
    <button
      type="button"
      className={className}
      data-app-action={component.appAction ?? undefined}
      aria-pressed={props.pressed}
      style={props.style}
      onClick={props.onClick}
    >
      {image !== null && <img className="plinth-button__image" src={image} alt="" draggable={false} />}
      {icon !== null && <img className="plinth-button__icon" src={icon} alt="" draggable={false} />}
      <span className={component.hideLabel === true ? "plinth-visually-hidden" : "plinth-button__label"}>{props.label}</span>
    </button>
  );
}

/**
 * One page of buttons. The outer element animates the page change with a
 * transform string, and carries the page's background, so it slides with the
 * page; the inner one is the draggable surface, because Motion's drag moves
 * `x` and a transform string on the same element would cancel it. The
 * background is the one Rust resolved for page `pageIndex` of the view.
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
  const images = usePanelStore((s) => s.images);
  const backgroundId = usePanelStore((s) => s.view?.pages[props.pageIndex]?.backgroundId);
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
          style={backgroundOf(imageOf(images, backgroundId))}
        >
          <motion.div
            className="page-surface"
            style={{ gridTemplateRows: tracks(props.rows), gridTemplateColumns: tracks(props.cols) }}
            drag={props.locked ? false : "x"}
            dragConstraints={{ left: 0, right: 0 }}
            dragElastic={0.2}
            // No momentum past where the finger let go, and a firm spring back.
            dragMomentum={false}
            dragTransition={snapBack}
            onDragStart={() => guard.dragStarted()}
            onDragEnd={(_, info) => {
              guard.dragEnded();
              handleSwipeEnd(info, sectionRef.current?.getBoundingClientRect().width ?? 0, props.locked, (d) => void navigate(d));
            }}
          >
            {props.components.map((component) => (
              <PanelButton
                key={component.id}
                component={component}
                label={component.label ?? component.id}
                allowImage
                // Its own colours, if its `style` gives any, over the theme
                // the panel sets: only on the button, so its neighbours keep it.
                style={{ ...themeStyle(component.style ?? {}), ...cellsOf(component) }}
                onClick={() => {
                  if (!guard.shouldIgnoreClick()) void press(component.id);
                }}
              />
            ))}
          </motion.div>
        </motion.div>
      </AnimatePresence>
    </section>
  );
}
