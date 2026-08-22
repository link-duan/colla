import { expect, test } from "@playwright/test"

let consoleErrors = []

test.beforeEach(async ({ page }) => {
  consoleErrors = []
  page.on("pageerror", error => consoleErrors.push(error.message))
  page.on("console", message => {
    if (message.type() === "error") consoleErrors.push(message.text())
  })
})

test.afterEach(async () => {
  expect(consoleErrors).toEqual([])
})

test("runs the public API on the browser main thread", async ({ page }) => {
  await page.goto("/")
  await expect.poll(() => page.evaluate(() => globalThis.collaResult)).toBe("after")
})

test("runs the public API in a Dedicated Worker", async ({ page }) => {
  await page.goto("/")
  const result = await page.evaluate(() => new Promise((resolve, reject) => {
    const worker = new Worker(new URL("/dedicated-worker.js", location.href), {
      type: "module",
    })
    worker.addEventListener("message", event => {
      worker.terminate()
      resolve(event.data)
    }, { once: true })
    worker.addEventListener("error", event => {
      worker.terminate()
      reject(new Error(event.message))
    }, { once: true })
  }))
  expect(result).toBe("after")
})

test("runs the public API in a Shared Worker", async ({ page }) => {
  await page.goto("/")
  const supportsSharedWorker = await page.evaluate(
    () => typeof SharedWorker !== "undefined",
  )
  test.skip(!supportsSharedWorker, "host does not support SharedWorker")
  const result = await page.evaluate(() => new Promise((resolve, reject) => {
    const worker = new SharedWorker(new URL("/shared-worker.js", location.href), {
      type: "module",
    })
    worker.port.addEventListener("message", event => {
      worker.port.close()
      resolve(event.data)
    }, { once: true })
    worker.port.addEventListener("messageerror", () => {
      worker.port.close()
      reject(new Error("Shared Worker returned an unreadable message"))
    }, { once: true })
    worker.addEventListener("error", event => {
      worker.port.close()
      reject(new Error(event.message))
    }, { once: true })
    worker.port.start()
  }))
  expect(result).toBe("after")
})
