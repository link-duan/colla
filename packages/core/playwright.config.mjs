import { defineConfig } from "@playwright/test"
import { readFileSync } from "node:fs"
import { join } from "node:path"

const pointerFile = join(import.meta.dirname, "e2e/.fixture-dir")
const fixtureDir = readFileSync(pointerFile, "utf8").trim()
if (fixtureDir === "") {
  throw new Error("fixture dir is empty; run pnpm test:e2e")
}

const allBrowsers = ["chromium", "firefox", "webkit"]
const requested = (process.env.COLLA_BROWSERS ?? allBrowsers.join(","))
  .split(",")
  .map(name => name.trim())
  .filter(name => name !== "")
for (const name of requested) {
  if (!allBrowsers.includes(name)) {
    throw new Error(`unknown browser "${name}"; expected one of ${allBrowsers.join(", ")}`)
  }
}

export default defineConfig({
  testDir: "./e2e",
  testMatch: "browser.spec.mjs",
  outputDir: `${fixtureDir}/test-results`,
  workers: 1,
  forbidOnly: !!process.env.CI,
  reporter: "line",
  use: {
    baseURL: "http://127.0.0.1:4173",
  },
  projects: requested.map(name => ({ name, use: { browserName: name } })),
  webServer: {
    command: "npm exec vite -- --host 127.0.0.1 --port 4173 --strictPort",
    cwd: fixtureDir,
    url: "http://127.0.0.1:4173",
    reuseExistingServer: false,
    timeout: 120_000,
  },
})
