import { defineConfig, devices } from "@playwright/test";

const e2eUrl = process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: e2eUrl,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    ...devices["Desktop Chrome"],
  },
  webServer: {
    command: `pnpm exec vite --host 127.0.0.1 --mode e2e --port ${new URL(e2eUrl).port || "80"}`,
    url: e2eUrl,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
