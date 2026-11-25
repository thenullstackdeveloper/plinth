import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// The test projects live in vitest.config.ts.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
  },
});
