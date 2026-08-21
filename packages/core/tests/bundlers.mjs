import assert from "node:assert/strict"
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"
import { pathToFileURL, fileURLToPath } from "node:url"
import { execFileSync } from "node:child_process"
import { createRequire } from "node:module"
import { test } from "node:test"

const packageDir = resolve(fileURLToPath(new URL("..", import.meta.url)))

test("bundler integration", async () => {
const packageJson = JSON.parse(await readFile(join(packageDir, "package.json"), "utf8"))
const fixtureDir = await mkdtemp(join(tmpdir(), "colla-bundlers-"))
const packageSpec = process.env.COLLA_PACKAGE_SPEC

async function writeFixture(name, extra = "") {
  await writeFile(join(fixtureDir, name), `
    import { apply, Change, Value } from "colla-ot"
    const base = Value.fromJS("before")
    const change = Change.build(builder => builder.replace("after"))
    const next = apply(base, change)
    if (next.toJS() !== "after") throw new Error("Colla tracer failed")
    base.dispose()
    change.dispose()
    next.dispose()
    export const result = "after"
    ${extra}
  `)
}

try {
  let installSpec = packageSpec
  if (installSpec === undefined) {
    execFileSync("pnpm", ["pack", "--pack-destination", fixtureDir], {
      cwd: packageDir,
      stdio: "inherit",
    })
    installSpec = join(fixtureDir, `colla-ot-${packageJson.version}.tgz`)
  }
  await writeFile(join(fixtureDir, "package.json"), JSON.stringify({ type: "module" }))
  execFileSync("npm", ["install", "--ignore-scripts", "--save-exact", installSpec], {
    cwd: fixtureDir,
    stdio: "inherit",
  })
  execFileSync("npm", [
    "install",
    "--no-save",
    `vite@${process.env.COLLA_VITE_VERSION ?? "5.4.19"}`,
    `rollup@${process.env.COLLA_ROLLUP_VERSION ?? "4.46.2"}`,
    `@rollup/plugin-node-resolve@${process.env.COLLA_NODE_RESOLVE_VERSION ?? "16.0.1"}`,
  ], {
    cwd: fixtureDir,
    stdio: "inherit",
  })
  const fixtureRequire = createRequire(join(fixtureDir, "package.json"))
  const vitePackageDir = dirname(fixtureRequire.resolve("vite/package.json"))
  const vite = await import(pathToFileURL(join(vitePackageDir, "dist/node/index.js")))
  const rollupModule = await import(pathToFileURL(fixtureRequire.resolve("rollup")))
  const resolveModule = await import(pathToFileURL(
    fixtureRequire.resolve("@rollup/plugin-node-resolve"),
  ))
  const viteApi = vite
  const rollupApi = rollupModule.rollup === undefined ? rollupModule.default : rollupModule
  const { build: viteBuild, createServer } = viteApi
  const { rollup } = rollupApi
  const nodeResolve = resolveModule.nodeResolve ?? resolveModule.default

  await writeFixture("main.js")
  await writeFixture("ssr.js")
  await writeFixture("dedicated-worker.js", "globalThis.workerResult = result")
  await writeFixture("shared-worker.js", "globalThis.sharedWorkerResult = result")
  await writeFile(
    join(fixtureDir, "index.html"),
    '<script type="module" src="/main.js"></script>',
  )

  const server = await createServer({ root: fixtureDir, logLevel: "error" })
  try {
    await server.listen()
    const response = await fetch(`${server.resolvedUrls.local[0]}main.js`)
    assert.equal(response.ok, true)
    assert.match(await response.text(), /colla-ot|\.vite\/deps/)
    const ssr = await server.ssrLoadModule("/ssr.js")
    assert.equal(ssr.result, "after")
  } finally {
    server.httpServer?.closeAllConnections?.()
    await Promise.race([
      server.close(),
      new Promise(resolveClose => setTimeout(resolveClose, 100)),
    ])
  }

  const viteOut = join(fixtureDir, "vite-dist")
  await viteBuild({
    root: fixtureDir,
    logLevel: "error",
    build: {
      outDir: viteOut,
      emptyOutDir: true,
      lib: { entry: join(fixtureDir, "main.js"), formats: ["es"], fileName: "main" },
    },
  })
  const viteModule = await import(`${pathToFileURL(join(viteOut, "main.js"))}?v=${Date.now()}`)
  assert.equal(viteModule.result, "after")

  for (const entry of ["main.js", "dedicated-worker.js", "shared-worker.js"]) {
    const bundle = await rollup({
      input: join(fixtureDir, entry),
      plugins: [nodeResolve({ browser: true })],
    })
    const output = join(fixtureDir, `rollup-${entry}`)
    await bundle.write({ file: output, format: "es" })
    await bundle.close()
    const module = await import(`${pathToFileURL(output)}?v=${Date.now()}`)
    assert.equal(module.result, "after")
    if (entry === "dedicated-worker.js") assert.equal(globalThis.workerResult, "after")
    if (entry === "shared-worker.js") assert.equal(globalThis.sharedWorkerResult, "after")
  }

  const installedPackage = JSON.parse(
    await readFile(
      join(fixtureDir, "node_modules/colla-ot/package.json"),
      "utf8",
    ),
  )
  if (process.env.COLLA_EXPECTED_PACKAGE_VERSION !== undefined) {
    assert.equal(installedPackage.version, process.env.COLLA_EXPECTED_PACKAGE_VERSION)
  }
  assert.deepEqual(Object.keys(installedPackage.exports), ["."])
  const installedDist = join(fixtureDir, "node_modules/colla-ot/dist")
  assert.ok((await readdir(installedDist)).includes("browser.js"))
  const wasmBytes = await readFile(join(installedDist, "internal/colla_wasm_bg.wasm"))
  const base64Module = await import(
    `${pathToFileURL(join(installedDist, "internal/wasm_base64.js"))}?v=${Date.now()}`
  )
  assert.deepEqual(Buffer.from(base64Module.default, "base64"), wasmBytes)
} finally {
  await rm(fixtureDir, { recursive: true, force: true })
}
})
