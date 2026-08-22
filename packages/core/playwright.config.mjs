import { defineConfig } from "@playwright/test"
import { readFileSync } from "node:fs"
import { join } from "node:path"

const pointerFile = join(import.meta.dirname, "e2e/.fixture-dir")
const fixtureDir = readFileSync(pointerFile, "utf8").trim()
if (fixtureDir === "") {
  throw new Error("fixture dir is empty; run pnpm test:e2e")
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
