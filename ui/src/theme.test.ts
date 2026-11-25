import { expect, test } from "vitest";

import { themeStyle } from "./theme";

test("themeStyle keeps the --plinth- variables", () => {
  expect(
    themeStyle({
      "--plinth-color-button-background": "#262a3b",
      "--plinth-font-family-label": "system-ui, Segoe UI, sans-serif",
    }),
  ).toEqual({
    "--plinth-color-button-background": "#262a3b",
    "--plinth-font-family-label": "system-ui, Segoe UI, sans-serif",
  });
});

test("themeStyle drops names outside --plinth-", () => {
  expect(themeStyle({ "--evil": "#000000", color: "#000000", "--plinth-X": "#000000" })).toEqual({});
});

test("themeStyle drops values that could carry CSS", () => {
  const style = themeStyle({
    "--plinth-a": "url(x)",
    "--plinth-b": "red;color:blue",
    "--plinth-c": "var(--a)",
    "--plinth-d": "'a'",
    "--plinth-ok": "#1e1e2e",
  });
  expect(style).toEqual({ "--plinth-ok": "#1e1e2e" });
});
