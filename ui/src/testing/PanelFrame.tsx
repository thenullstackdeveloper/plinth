import type { ReactNode } from "react";

import type { ThemeVars } from "../adapters/panelApi";
import { themeStyle } from "../theme";
import defaultTheme from "./fixtures/default-theme.json";

/**
 * The panel window around a story: a `.panel` of the window's size
 * (900×300, `PanelWindowSpec` in crates/plinth-tauri/src/window.rs) carrying a
 * theme, so a component alone sees the same CSS variables it gets in the app.
 */
export function PanelFrame({ children, theme = defaultTheme }: { children: ReactNode; theme?: ThemeVars }) {
  return (
    <div className="panel" style={{ width: 900, height: 300, ...themeStyle(theme) }}>
      {children}
    </div>
  );
}
