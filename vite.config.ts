// `defineConfig` comes from vitest/config so the `test` block typechecks;
// it re-exports vite's own and behaves identically for the build.
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Tauri serves the frontend from a fixed port in development and from bundled
// assets in production. It expects a stable port and no automatic fallback.
//
// The test config lives here rather than in a separate vitest.config.ts: vitest
// ships its own copy of vite, and two config files means two incompatible
// `Plugin` types.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2021",
    sourcemap: false,
    outDir: "dist",
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}", "widgets/**/*.test.ts"],
  },
});
