import { usePanelStore } from "../store/panelStore";
import { themeStyle } from "../theme";
import { PageView } from "./PageView";
import { Sidebar } from "./Sidebar";

/** The panel layout: the sidebar on its side, and the current page, under the template's theme. */
export function Panel() {
  const view = usePanelStore((s) => s.view);
  const lastDirection = usePanelStore((s) => s.lastDirection);
  const page = view?.pages[view.currentPage];
  if (!view || !page) return null;

  const sidebar = <Sidebar components={view.sidebar} locked={view.locked} />;
  return (
    <div
      className="panel"
      data-side={view.sidebarSide}
      data-locked={String(view.locked)}
      style={themeStyle(view.theme)}
    >
      {view.sidebarSide === "left" && sidebar}
      <PageView
        components={page.components}
        rows={page.rows}
        cols={page.cols}
        pageIndex={view.currentPage}
        direction={lastDirection}
        locked={view.locked}
      />
      {view.sidebarSide === "right" && sidebar}
    </div>
  );
}
