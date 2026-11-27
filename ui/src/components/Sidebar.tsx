import type { ComponentDto } from "../adapters/panelApi";
import { usePanelStore } from "../store/panelStore";
import { PanelButton } from "./PageView";

/** The sidebar buttons, with an icon but never an image. The lock button shows the action it will take. */
export function Sidebar(props: { components: ComponentDto[]; locked: boolean }) {
  const press = usePanelStore((s) => s.press);

  return (
    <aside className="sidebar" aria-label="Sidebar">
      {props.components.map((component) => {
        const isLock = component.appAction === "toggle_lock";
        const label = isLock ? (props.locked ? "Unlock" : "Lock") : (component.label ?? component.id);
        return (
          <PanelButton
            key={component.id}
            component={component}
            label={label}
            allowImage={false}
            pressed={isLock ? props.locked : undefined}
            onClick={() => void press(component.id)}
          />
        );
      })}
    </aside>
  );
}
