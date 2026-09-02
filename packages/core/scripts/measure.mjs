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

execFileSync("pnpm", ["build"], { cwd: packageDir, stdio: "inherit" })

const {
  Change,
  ValueHandle,
  apply,
  compose,
  transformPair,
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

const input = {
  count: 1n,
  meta: { status: "draft" },
  items: ["a", "b", "c"],
}
const base = ValueHandle.fromJS(input)
const first = Change.build(change => change.map(map =>
  map.modify("count", value => value.intAdd(1n))))
const afterFirst = apply(base, first)
const second = Change.build(change => change.map(map =>
  map.modify("count", value => value.intAdd(2n))))
const concurrent = Change.build(change => change.map(map =>
  map.modify("count", value => value.intAdd(3n))))

const timings = {
  valueFromJS: benchmark(200, () => {
    const value = ValueHandle.fromJS(input)
    value.toJS()
    value.dispose()
  }),
  builder: benchmark(200, () => {
    const change = Change.build(change => change.map(map =>
      map.modify("meta", meta => meta.map(value =>
        value.modify("status", status => status.replace("ready"))))))
    change.dispose()
  }),
  apply: benchmark(500, () => {
    const value = apply(base, first)
    value.dispose()
  }),
  compose: benchmark(500, () => {
    const change = compose(first, second)
    change.dispose()
  }),
  transformPair: benchmark(500, () => {
    const pair = transformPair(first, concurrent, { order: "left-first" })
    pair[0].dispose()
    pair[1].dispose()
  }),
}

for (const handle of [base, first, afterFirst, second, concurrent]) handle.dispose()

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
    stdio: "inherit",
  })
  const tarballName = `${packageJson.name.replace(/^@/, "").replace("/", "-")}-${packageJson.version}.tgz`
  const tarball = await readFile(join(fixtureDir, tarballName))
  const wasmPath = resolve(packageDir, "dist/internal/colla_wasm_bg.wasm")
  const browserBase64Path = resolve(packageDir, "dist/internal/wasm_base64.js")
  const wasm = await readFile(wasmPath)
  const browserBase64 = await readFile(browserBase64Path)
  const result = {
    version: packageJson.version,
    environment: {
      node: process.version,
      platform: process.platform,
      arch: process.arch,
    },
    sizes: {
      wasm: compressedSizes(wasm),
      browserBase64: compressedSizes(browserBase64),
      npmTarball: compressedSizes(tarball),
    },
    milliseconds: {
      synchronousInitialization: median(initSamples),
      ...timings,
    },
  }
  const outputIndex = process.argv.indexOf("--output")
  if (outputIndex >= 0) {
    const output = resolve(process.cwd(), process.argv[outputIndex + 1])
    await mkdir(dirname(output), { recursive: true })
    await writeFile(output, `${JSON.stringify(result, null, 2)}\n`)
  }
  console.log(JSON.stringify(result, null, 2))
} finally {
  await rm(fixtureDir, { recursive: true, force: true })
}
