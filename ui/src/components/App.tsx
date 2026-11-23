import { useEffect } from "react";

import { usePanelStore } from "../store/panelStore";
import { PageView } from "./PageView";

export function App() {
  const view = usePanelStore((s) => s.view);
  const error = usePanelStore((s) => s.error);
  const lastDirection = usePanelStore((s) => s.lastDirection);
  const load = usePanelStore((s) => s.load);

  useEffect(() => {
    void load();
  }, [load]);

  const page = view?.pages[view.currentPage];

  return (
    <main>
      {view && page && (
        <PageView components={page.components} pageIndex={view.currentPage} direction={lastDirection} />
      )}
      {error !== null && <p role="alert">{error}</p>}
    </main>
  );
}
