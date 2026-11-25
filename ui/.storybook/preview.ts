import type { Preview } from "@storybook/react-vite";
import { sb } from "storybook/test";

import "../src/panel.css";

// The Tauri bridge does not exist outside the webview: every export becomes a mock.
sb.mock(import("@tauri-apps/api/core"));
sb.mock(import("@tauri-apps/api/event"));

const preview: Preview = {
  parameters: {
    // Fail the story test on accessibility violations.
    a11y: { test: "error" },
  },
};

export default preview;
