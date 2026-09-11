import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join, resolve } from "node:path"
import { test } from "node:test"
import { tracer } from "./public-trace.mjs"
const packageDir = resolve(import.meta.dirname, "..")
test("packed package has zero dependencies and works outside the workspace", async () => {
  const directory = await mkdtemp(join(tmpdir(), "colla-node-"))
  try {
    const pkg = JSON.parse(await readFile(join(packageDir, "package.json"), "utf8"))
    assert.deepEqual(pkg.dependencies ?? {}, {})
    if (!process.env.COLLA_PACKAGE_SPEC) execFileSync("pnpm", ["pack", "--pack-destination", directory], { cwd: packageDir, stdio: "inherit" })
    await writeFile(join(directory, "package.json"), JSON.stringify({ type: "module" }))
    execFileSync("npm", ["install", "--ignore-scripts", "--save-exact", process.env.COLLA_PACKAGE_SPEC ?? join(directory, `colla-ot-${pkg.version}.tgz`)], { cwd: directory, stdio: "inherit" })
    await writeFile(join(directory, "trace.mjs"), tracer + '\nif (trace() !== "after") throw Error("trace failed")\n')
    execFileSync(process.execPath, ["trace.mjs"], { cwd: directory, stdio: "inherit" })
    await writeFile(join(directory, "types.ts"), `
      import { Document, History, Value, ElementId, Change, Transaction, transform, text, ref } from "colla-ot"
      const doc = Document.create({ title: text("Hi"), chosen: null })
      const id: ElementId = doc.idAt(["title"])
      const result = doc.edit(tx => { tx.text(id).insert(2, "!"); tx.set(["chosen"], ref(id)) })
      if (result) { const [left, right]: readonly [Change, Change] = transform(result.before, result.change, Change.noop(), { priority: "left" }); void [left, right] }
      const snapshot: Value = doc.snapshot()
      History.attach(doc).undo()
      // @ts-expect-error transactions are scope-owned and cannot be constructed
      new Transaction()
      void snapshot
    `)
    execFileSync(process.execPath, [join(packageDir, "node_modules/typescript/bin/tsc"), "--noEmit", "--strict", "--target", "ES2022", "--module", "NodeNext", "--moduleResolution", "NodeNext", "types.ts"], { cwd: directory, stdio: "inherit" })
  } finally { await rm(directory, { recursive: true, force: true }) }
})
