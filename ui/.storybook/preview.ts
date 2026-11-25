import type { Preview } from "@storybook/react-vite";
import { createElement } from "react";
import { sb } from "storybook/test";

import "../src/panel.css";
import { PanelFrame } from "../src/testing/PanelFrame";

// The Tauri bridge does not exist outside the webview: every export becomes a mock.
sb.mock(import("@tauri-apps/api/core"));
sb.mock(import("@tauri-apps/api/event"));

const preview: Preview = {
  // Every story inside the panel window with the default theme. This file is
  // not .tsx, hence createElement.
  decorators: [(Story) => createElement(PanelFrame, null, createElement(Story))],
  parameters: {
    // Fail the story test on accessibility violations.
    a11y: { test: "error" },
  },
};

export default preview;
