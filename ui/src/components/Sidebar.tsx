import type { ComponentDto } from "../adapters/panelApi";
import { usePanelStore } from "../store/panelStore";

/** The sidebar buttons. The lock button shows the action it will take. */
export function Sidebar(props: { components: ComponentDto[]; locked: boolean }) {
  const press = usePanelStore((s) => s.press);

  return (
    <aside aria-label="Sidebar">
      {props.components.map((component) => {
        const isLock = component.appAction === "toggle_lock";
        const label = isLock ? (props.locked ? "Unlock" : "Lock") : (component.label ?? component.id);
        return (
          <button
            key={component.id}
            type="button"
            aria-pressed={isLock ? props.locked : undefined}
            onClick={() => void press(component.id)}
          >
            {label}
          </button>
        );
      })}
    </aside>
  );
}
