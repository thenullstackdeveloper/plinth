import { defineConfig, devices } from "@playwright/test";

/**
 * End-to-end tests of the built UI in Chromium, the engine of WebView2 (the
 * panel's webview on Windows). They cover what jsdom cannot: layout, hit
 * testing, text selection and real mouse drags. The Rust backend is replaced
 * by `e2e/fakeBackend.ts`; its logic has its own Rust tests.
 */
export default defineConfig({
  testDir: "e2e",
  testMatch: "*.e2e.ts",
  forbidOnly: true,
  reporter: "list",
  use: {
    ...devices["Desktop Chrome"],
    baseURL: "http://127.0.0.1:4173",
    // The panel window: `PanelWindowSpec` in crates/plinth-tauri/src/window.rs.
    viewport: { width: 900, height: 300 },
  },
  projects: [{ name: "chromium" }],
  webServer: {
    // The production build, so the stylesheet import in main.tsx is covered too.
    command: "pnpm build && pnpm exec vite preview --host 127.0.0.1 --port 4173 --strictPort",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: false,
  },
});
