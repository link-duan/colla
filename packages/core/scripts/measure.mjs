import { rollup } from "rollup"
import { execFileSync } from "node:child_process"
import { brotliCompressSync, gzipSync } from "node:zlib"
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"

const packageDir = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const scriptPath = fileURLToPath(import.meta.url)

if (process.argv[2] === "--init") {
  const started = process.hrtime.bigint()
  await import(`${pathToFileURL(resolve(packageDir, "dist/node.js"))}?init=${process.pid}`)
  const elapsed = Number(process.hrtime.bigint() - started) / 1_000_000
  process.stdout.write(JSON.stringify(elapsed))
  process.exit(0)
}

execFileSync("pnpm", ["build"], {
  cwd: packageDir,
  stdio: ["inherit", process.stderr, process.stderr],
})

const {
  Change,
  Value,
  Document,
  apply,
  compose,
  transform,
} = await import(pathToFileURL(resolve(packageDir, "dist/node.js")))

function median(values) {
  const sorted = [...values].sort((left, right) => left - right)
  return sorted[Math.floor(sorted.length / 2)]
}

function benchmark(iterations, operation) {
  for (let index = 0; index < Math.min(iterations, 20); index += 1) operation()
  const samples = []
  for (let sample = 0; sample < 5; sample += 1) {
    const started = process.hrtime.bigint()
    for (let index = 0; index < iterations; index += 1) operation()
    samples.push(Number(process.hrtime.bigint() - started) / 1_000_000 / iterations)
  }
  return median(samples)
}

function compressedSizes(bytes) {
  return {
    raw: bytes.byteLength,
    gzip: gzipSync(bytes).byteLength,
    brotli: brotliCompressSync(bytes).byteLength,
  }
}

function formatBytes(bytes) {
  const units = ["B", "KiB", "MiB", "GiB"]
  let value = bytes
  let unitIndex = 0
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }
  return `${value.toFixed(unitIndex === 0 ? 0 : 1)} ${units[unitIndex]}`
}

function formatMilliseconds(milliseconds) {
  const digits = milliseconds >= 10 ? 2 : milliseconds >= 1 ? 3 : 4
  return `${milliseconds.toFixed(digits)} ms`
}

function formatMarkdown(result, json) {
  const sizeRows = [
    ["Wasm binary", result.sizes.wasm],
    ["Browser base64 module", result.sizes.browserBase64],
    ["Complete browser ESM bundle", result.sizes.browserEntry],
    ["npm tarball", result.sizes.npmTarball],
  ].map(([name, sizes]) =>
    `| ${name} | ${formatBytes(sizes.raw)} | ${formatBytes(sizes.gzip)} | ${formatBytes(sizes.brotli)} |`)

  const timingRows = [
    ["Synchronous initialization", result.milliseconds.synchronousInitialization],
    ["Value.fromJS + toJS", result.milliseconds.valueFromJS],
    ["Transaction", result.milliseconds.builder],
    ["Apply", result.milliseconds.apply],
    ["Compose", result.milliseconds.compose],
    ["Encode", result.milliseconds.encode],
    ["Decode", result.milliseconds.decode],
    ["Transform", result.milliseconds.transform],
  ].map(([name, milliseconds]) => `| ${name} | ${formatMilliseconds(milliseconds)} |`)

  return [
    "## Quality baseline",
    "",
    `**colla-ot ${result.version}** · ${result.environment.node} · ${result.environment.platform}/${result.environment.arch}`,
    "",
    "### Artifact sizes",
    "",
    "| Artifact | Raw | Gzip | Brotli |",
    "| --- | ---: | ---: | ---: |",
    ...sizeRows,
    "",
    "### Performance",
    "",
    "| Measurement | Median time |",
    "| --- | ---: |",
    ...timingRows,
    "",
    "Initialization is measured across fresh imports. Operation timings are median milliseconds per call.",
    "",
    "<details>",
    "<summary>Raw JSON</summary>",
    "",
    "```json",
    json.trimEnd(),
    "```",
    "",
    "</details>",
    "",
  ].join("\n")
}

async function writeOutput(flag, contents) {
  const index = process.argv.indexOf(flag)
  if (index < 0) return
  const path = process.argv[index + 1]
  if (!path) throw new Error(`${flag} requires a path`)
  const output = resolve(process.cwd(), path)
  await mkdir(dirname(output), { recursive: true })
  await writeFile(output, contents)
}

const input = {
  count: 1n,
  meta: { status: "draft" },
  items: ["a", "b", "c"],
}
const base = Value.fromJS(input)
const counter = base.idAt(["count"])
const first = Change.create([{ type: "add", target: counter, delta: 1n }])
const second = Change.create([{ type: "add", target: counter, delta: 2n }])
const concurrent = Change.create([{ type: "add", target: counter, delta: 3n }])
const bytes = base.encode()
const timings = {
  valueFromJS: benchmark(200, () => Value.fromJS(input).toJS()),
  builder: benchmark(200, () => { const doc = Document.create(base); doc.edit(tx => tx.set(["meta", "status"], "ready")); doc.close() }),
  apply: benchmark(500, () => apply(base, first)),
  compose: benchmark(500, () => compose(base, first, second)),
  transform: benchmark(500, () => transform(base, first, concurrent, { priority: "left" })),
  encode: benchmark(500, () => base.encode()),
  decode: benchmark(500, () => Value.decode(bytes)),
}
const memory = JSON.parse(execFileSync(process.execPath, ["--expose-gc", resolve(packageDir, "tests/memory.mjs")], { cwd: packageDir, encoding: "utf8" }))

const initSamples = Array.from({ length: 5 }, () => Number(JSON.parse(execFileSync(
  process.execPath,
  [scriptPath, "--init"],
  { cwd: packageDir, encoding: "utf8" },
))))

const packageJson = JSON.parse(await readFile(resolve(packageDir, "package.json"), "utf8"))
const fixtureDir = await mkdtemp(join(tmpdir(), "colla-measure-"))
try {
  execFileSync("pnpm", ["pack", "--pack-destination", fixtureDir], {
    cwd: packageDir,
    stdio: ["inherit", process.stderr, process.stderr],
  })
  const tarballName = `${packageJson.name.replace(/^@/, "").replace("/", "-")}-${packageJson.version}.tgz`
  const tarball = await readFile(join(fixtureDir, tarballName))
  const wasmPath = resolve(packageDir, "src/internal/colla_wasm_bg.wasm")
  const browserBase64Path = resolve(packageDir, "dist/internal/wasm_base64.js")
  const wasm = await readFile(wasmPath)
  const browserBase64 = await readFile(browserBase64Path)
  const bundle = await rollup({ input: resolve(packageDir, "dist/browser.js"), treeshake: false })
  const generated = await bundle.generate({ format: "es" })
  await bundle.close()
  const browserEntry = Buffer.from(generated.output.map(chunk => chunk.code ?? "").join("\n"))
  const result = {
    version: packageJson.version,
    environment: {
      node: process.version,
      platform: process.platform,
      arch: process.arch,
    },
    memory,
    sizes: {
      wasm: compressedSizes(wasm),
      browserBase64: compressedSizes(browserBase64),
      browserEntry: compressedSizes(browserEntry),
      npmTarball: compressedSizes(tarball),
    },
    milliseconds: {
      synchronousInitialization: median(initSamples),
      ...timings,
    },
  }
  const json = `${JSON.stringify(result, null, 2)}\n`
  const markdown = formatMarkdown(result, json)
  await writeOutput("--output", json)
  await writeOutput("--markdown-output", markdown)
  process.stdout.write(markdown)
  const budgets = JSON.parse(await readFile(resolve(packageDir, "size-budget.json"), "utf8"))
  const exceeded = []
  for (const [artifact, limits] of Object.entries(budgets)) {
    for (const [encoding, limit] of Object.entries(limits)) {
      const actual = result.sizes[artifact]?.[encoding]
      if (!Number.isSafeInteger(limit) || limit <= 0 || !Number.isSafeInteger(actual)) {
        throw new Error(`Invalid artifact size budget: ${artifact}.${encoding}`)
      }
      if (actual > limit) exceeded.push(`${artifact}.${encoding}: ${actual} > ${limit} bytes`)
    }
  }
  if (exceeded.length) throw new Error(`Artifact size budget exceeded:\n${exceeded.join("\n")}`)
} finally {
  await rm(fixtureDir, { recursive: true, force: true })
}
