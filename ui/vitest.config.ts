import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import { playwright } from "@vitest/browser-playwright";
import { defineConfig, mergeConfig } from "vitest/config";

import viteConfig from "./vite.config.ts";

// Two projects: the jsdom unit tests and the stories run as tests in Chromium.
// The Storybook plugin reads `.storybook/` from the working directory (run from ui/).
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      projects: [
        {
          extends: true,
          test: {
            name: "unit",
            include: ["src/**/*.test.{ts,tsx}"],
            environment: "jsdom",
            globals: false,
            setupFiles: ["./src/test-setup.ts"],
          },
        },
        {
          extends: true,
          plugins: [storybookTest()],
          // Pre-bundle what the stories import, so the first run on a clean cache
          // does not re-optimize and reload in the middle of the run.
          optimizeDeps: {
            include: [
              "@tauri-apps/api/core",
              "@tauri-apps/api/event",
              "motion/react",
              "zustand",
              "react",
              "react-dom/client",
              "react/jsx-dev-runtime",
              "storybook/test",
            ],
          },
          test: {
            name: "storybook",
            browser: {
              enabled: true,
              headless: true,
              provider: playwright({}),
              instances: [{ browser: "chromium" }],
            },
          },
        },
      ],
    },
  }),
);
