import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  test: {
    environment: "jsdom",
    exclude: ["tests/e2e/**", "gnome-extension/tests/**", "node_modules/**", "dist/**"],
    setupFiles: ["./src/test/setup.ts"],
    globals: true,
  },
});
