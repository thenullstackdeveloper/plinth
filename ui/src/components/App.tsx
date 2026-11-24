import { useEffect } from "react";

import { onPanelChanged } from "../adapters/panelApi";
import { usePanelStore } from "../store/panelStore";
import { Panel } from "./Panel";

export function App() {
  const error = usePanelStore((s) => s.error);
  const load = usePanelStore((s) => s.load);

  useEffect(() => {
    void load();
  }, [load]);

  // Redraw when Rust changes the panel on its own (a template picked in the
  // tray). `listen` resolves later, so an unmount before that unsubscribes
  // as soon as it does.
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    void onPanelChanged(() => void load()).then((stop) => {
      if (cancelled) stop();
      else unlisten = stop;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [load]);

  return (
    <main>
      <Panel />
      {error !== null && <p role="alert">{error}</p>}
    </main>
  );
}
