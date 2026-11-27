import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

import tauriConf from "../crates/plinth-tauri/tauri.conf.json" with { type: "json" };

// The test projects live in vitest.config.ts.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
  },
  // The window's production policy, so the end-to-end tests run under it.
  preview: {
    headers: { "Content-Security-Policy": tauriConf.app.security.csp },
  },
});
