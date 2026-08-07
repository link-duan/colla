import { defineConfig } from "@playwright/test"

const fixtureDir = process.env.COLLA_E2E_FIXTURE_DIR
if (fixtureDir === undefined) {
  throw new Error("COLLA_E2E_FIXTURE_DIR is required; run pnpm test:e2e")
}

export default defineConfig({
  testDir: "./e2e",
  testMatch: "browser.spec.mjs",
  outputDir: `${fixtureDir}/test-results`,
  workers: 1,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4173",
    browserName: "chromium",
  },
  webServer: {
    command: "npm exec vite -- --host 127.0.0.1 --port 4173 --strictPort",
    cwd: fixtureDir,
    url: "http://127.0.0.1:4173",
    reuseExistingServer: false,
    timeout: 120_000,
  },
})
