import assert from "node:assert/strict"
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join, resolve } from "node:path"
import { pathToFileURL, fileURLToPath } from "node:url"
import { execFileSync } from "node:child_process"
import { nodeResolve } from "@rollup/plugin-node-resolve"
import { rollup } from "rollup"
import { build as viteBuild, createServer } from "vite"

const packageDir = resolve(fileURLToPath(new URL("..", import.meta.url)))
const fixtureDir = await mkdtemp(join(tmpdir(), "colla-bundlers-"))

async function writeFixture(name, extra = "") {
  await writeFile(join(fixtureDir, name), `
    import { apply, Value } from "@colla/core"
    const base = Value.fromJS("before")
    const change = base.change().replace([], "after").build()
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
  execFileSync("pnpm", ["pack", "--pack-destination", fixtureDir], {
    cwd: packageDir,
    stdio: "inherit",
  })
  const tarball = join(fixtureDir, "colla-core-0.1.0.tgz")
  await writeFile(join(fixtureDir, "package.json"), JSON.stringify({ type: "module" }))
  execFileSync("npm", ["install", "--ignore-scripts", tarball], {
    cwd: fixtureDir,
    stdio: "inherit",
  })

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
    assert.match(await response.text(), /@colla\/core|\.vite\/deps/)
    const ssr = await server.ssrLoadModule("/ssr.js")
    assert.equal(ssr.result, "after")
  } finally {
    await server.close()
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
  }

  const installedPackage = JSON.parse(
    await readFile(
      join(fixtureDir, "node_modules/@colla/core/package.json"),
      "utf8",
    ),
  )
  assert.deepEqual(Object.keys(installedPackage.exports), ["."])
  const installedDist = join(fixtureDir, "node_modules/@colla/core/dist")
  assert.ok((await readdir(installedDist)).includes("browser.js"))
  const wasmBytes = await readFile(join(installedDist, "internal/colla_wasm_bg.wasm"))
  const base64Module = await import(
    `${pathToFileURL(join(installedDist, "internal/wasm_base64.js"))}?v=${Date.now()}`
  )
  assert.deepEqual(Buffer.from(base64Module.default, "base64"), wasmBytes)
} finally {
  await rm(fixtureDir, { recursive: true, force: true })
}
