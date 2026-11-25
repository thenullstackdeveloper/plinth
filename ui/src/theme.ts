import type { CSSProperties } from "react";

import type { ThemeVars } from "./adapters/panelApi";

/** Only Plinth's own custom properties. */
const NAME = /^--plinth-[a-z0-9-]+$/;

/**
 * Only plain values: colours, numbers with units, font lists. No parentheses,
 * quotes, semicolons or braces, so no `url(`, `var(` or a second declaration.
 */
const VALUE = /^[#A-Za-z0-9., -]{1,64}$/;

/**
 * The theme as a React `style`. Rust already writes every value from typed
 * numbers; this is the last barrier, so whatever does not look like a token
 * never reaches the page.
 */
export function themeStyle(theme: ThemeVars): CSSProperties {
  const style: Record<string, string> = {};
  for (const [name, value] of Object.entries(theme)) {
    if (NAME.test(name) && VALUE.test(value)) style[name] = value;
  }
  return style as CSSProperties;
}
