// Compares Criterion medians saved as the `base` baseline with the latest `new`
// run and fails when any benchmark regresses beyond the threshold.
//
// Usage: node scripts/compare-bench.mjs [--threshold 0.25] [--markdown-output FILE]
import { appendFile, readdir, readFile } from "node:fs/promises"
import { join, resolve } from "node:path"
import { parseArgs } from "node:util"

const { values } = parseArgs({
  options: {
    dir: { type: "string", default: "target/criterion" },
    threshold: { type: "string", default: process.env.BENCH_THRESHOLD ?? "0.25" },
    "markdown-output": { type: "string" },
  },
})
const root = resolve(values.dir)
const threshold = Number(values.threshold)
if (!Number.isFinite(threshold) || threshold <= 0) throw new Error("--threshold must be positive")

async function median(directory, baseline) {
  try {
    const estimates = JSON.parse(
      await readFile(join(directory, baseline, "estimates.json"), "utf8"),
    )
    return estimates.median.point_estimate
  } catch (error) {
    if (error.code === "ENOENT") return undefined
    throw error
  }
}
function duration(ns) {
  if (ns >= 1e6) return `${(ns / 1e6).toFixed(2)} ms`
  if (ns >= 1e3) return `${(ns / 1e3).toFixed(2)} µs`
  return `${ns.toFixed(1)} ns`
}

const rows = []
const regressions = []
for (const entry of await readdir(root, { withFileTypes: true })) {
  if (!entry.isDirectory() || entry.name === "report") continue
  const directory = join(root, entry.name)
  const [before, after] = await Promise.all([median(directory, "base"), median(directory, "new")])
  if (before === undefined || after === undefined) {
    rows.push(
      `| ${entry.name} | ${before === undefined ? "—" : duration(before)} | ${after === undefined ? "—" : duration(after)} | new or removed |`,
    )
    continue
  }
  const change = after / before - 1
  const regressed = change > threshold
  if (regressed) regressions.push(entry.name)
  rows.push(
    `| ${entry.name} | ${duration(before)} | ${duration(after)} | ${change >= 0 ? "+" : ""}${(change * 100).toFixed(1)}%${regressed ? " ❌" : ""} |`,
  )
}
if (rows.length === 0) throw new Error(`No Criterion results under ${root}`)

const report = [
  `### Benchmarks (median, regression threshold +${(threshold * 100).toFixed(0)}%)`,
  "",
  "| Benchmark | Base | Head | Change |",
  "| --- | ---: | ---: | ---: |",
  ...rows.toSorted(),
  "",
].join("\n")
console.log(report)
if (values["markdown-output"]) await appendFile(values["markdown-output"], `${report}\n`)
if (regressions.length > 0) {
  console.error(`Benchmark regression beyond threshold: ${regressions.join(", ")}`)
  process.exit(1)
}
