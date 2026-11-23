import { usePanelStore } from "../store/panelStore";
import { PageView } from "./PageView";
import { Sidebar } from "./Sidebar";

/** The panel layout: the sidebar on its side, and the current page. */
export function Panel() {
  const view = usePanelStore((s) => s.view);
  const lastDirection = usePanelStore((s) => s.lastDirection);
  const page = view?.pages[view.currentPage];
  if (!view || !page) return null;

  const sidebar = <Sidebar components={view.sidebar} locked={view.locked} />;
  return (
    <div data-side={view.sidebarSide} data-locked={String(view.locked)}>
      {view.sidebarSide === "left" && sidebar}
      <PageView
        components={page.components}
        pageIndex={view.currentPage}
        direction={lastDirection}
        locked={view.locked}
      />
      {view.sidebarSide === "right" && sidebar}
    </div>
  );
}
