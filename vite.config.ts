import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    // Tauri debug loads http://127.0.0.1:1420. Binding the default IPv6-only
    // ::1 leaves that address refused on some Linux boxes (dogfood D12).
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  test: {
    environment: "jsdom",
    exclude: [
      "tests/e2e/**",
      "tests/release/**",
      "tests/showcase/**",
      "gnome-extension/tests/**",
      "node_modules/**",
      "dist/**",
    ],
    setupFiles: ["./src/test/setup.ts"],
    globals: true,
  },
});
