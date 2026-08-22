import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import {
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises"
import { tmpdir } from "node:os"
import { join, resolve } from "node:path"

const packageDir = resolve(import.meta.dirname, "..")
const pointerFile = join(import.meta.dirname, ".fixture-dir")
const temporaryRoot = await realpath(tmpdir())
const fixtureDir = await mkdtemp(join(temporaryRoot, "colla-browser-e2e-"))
await writeFile(pointerFile, fixtureDir)
const packageSpec = process.env.COLLA_PACKAGE_SPEC

const tracer = `
import { apply, Change, Value } from "colla-ot"

export function trace() {
  const base = Value.fromJS("before")
  const change = Change.build(change => change.replace("after"))
  const next = apply(base, change)
  try {
    return next.toJS()
  } finally {
    next.dispose()
    change.dispose()
    base.dispose()
  }
}
`

try {
  let installSpec = packageSpec
  if (installSpec === undefined) {
    execFileSync("pnpm", ["pack", "--pack-destination", fixtureDir], {
      cwd: packageDir,
      stdio: "inherit",
    })
    const archives = (await readdir(fixtureDir)).filter(name => name.endsWith(".tgz"))
    assert.equal(archives.length, 1, "expected pnpm pack to produce one archive")
    installSpec = join(fixtureDir, archives[0])
  }

  await writeFile(join(fixtureDir, "package.json"), `${JSON.stringify({
    private: true,
    type: "module",
  }, null, 2)}\n`)
  execFileSync("npm", [
    "install",
    "--ignore-scripts",
    "--save-exact",
    installSpec,
    `vite@${process.env.COLLA_VITE_VERSION ?? "5.4.19"}`,
  ], { cwd: fixtureDir, stdio: "inherit" })

  await writeFile(join(fixtureDir, "tracer.js"), tracer)
  await writeFile(join(fixtureDir, "main.js"), `
    import { trace } from "./tracer.js"
    globalThis.collaResult = trace()
  `)
  await writeFile(join(fixtureDir, "dedicated-worker.js"), `
    import { trace } from "./tracer.js"
    postMessage(trace())
  `)
  await writeFile(join(fixtureDir, "shared-worker.js"), `
    import { trace } from "./tracer.js"
    globalThis.onconnect = event => event.ports[0].postMessage(trace())
  `)
  await writeFile(
    join(fixtureDir, "index.html"),
    '<!doctype html><script type="module" src="/main.js"></script>',
  )

  const installedPackage = JSON.parse(await readFile(
    join(fixtureDir, "node_modules/colla-ot/package.json"),
    "utf8",
  ))
  if (process.env.COLLA_EXPECTED_PACKAGE_VERSION !== undefined) {
    assert.equal(installedPackage.version, process.env.COLLA_EXPECTED_PACKAGE_VERSION)
  }

  execFileSync("pnpm", ["exec", "playwright", "test", "--config", "playwright.config.mjs"], {
    cwd: packageDir,
    stdio: "inherit",
  })
} finally {
  await rm(fixtureDir, { recursive: true, force: true })
  await rm(pointerFile, { force: true })
}
