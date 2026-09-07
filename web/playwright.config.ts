import { defineConfig, devices } from "@playwright/test";

/**
 * End-to-end smoke tests against the sandbox stack: a copy of the real database served by the
 * debug backend on :3011 with a Vite dev server on :3003 in front of it (see `e2e/sandbox.sh`).
 * Never point these at the real app on :3000 — the specs sign in, open dialogs and save.
 */
const PORT = 3003;
export const BASE_URL = process.env.LIBRARIAN_E2E_URL ?? `http://127.0.0.1:${PORT}`;
export const CREDENTIALS = {
  username: process.env.LIBRARIAN_E2E_USER ?? "toby",
  password: process.env.LIBRARIAN_E2E_PASSWORD ?? "sandbox-password-123",
};
export default defineConfig({
  testDir: "./e2e/tests",
  outputDir: "./e2e/.results",
  fullyParallel: false,
  workers: 1,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  timeout: 45_000,
  expect: { timeout: 10_000 },
  reporter: process.env.CI ? [["github"], ["list"]] : [["list"]],
  use: {
    baseURL: BASE_URL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
    colorScheme: "dark",
  },
  projects: [
    {
      name: "signed-out",
      testMatch: /signin\.spec\.ts$/,
      use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } },
    },
    {
      name: "desktop",
      testIgnore: /signin\.spec\.ts$/,
      use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } },
    },
  ],
  webServer: {
    command: "e2e/sandbox.sh start",
    url: BASE_URL,
    reuseExistingServer: true,
    timeout: 120_000,
    stdout: "ignore",
    stderr: "pipe",
  },
});
