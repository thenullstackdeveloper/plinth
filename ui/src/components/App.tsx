import { useEffect } from "react";

import { usePanelStore } from "../store/panelStore";
import { Panel } from "./Panel";

export function App() {
  const error = usePanelStore((s) => s.error);
  const load = usePanelStore((s) => s.load);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <main>
      <Panel />
      {error !== null && <p role="alert">{error}</p>}
    </main>
  );
}
