import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import { mkdtemp, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const packageDir = resolve(fileURLToPath(new URL("..", import.meta.url)))
const fixtureDir = await mkdtemp(join(tmpdir(), "colla-node-tracer-"))

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

  const fixture = `
    import assert from "node:assert/strict"
    import {
      apply,
      Change,
      CollaError,
      DEFAULT_INPUT_LIMITS,
      int,
      Value,
    } from "@colla/core"

    const nullPrototype = Object.create(null)
    nullPrototype.__proto__ = "data"
    nullPrototype.nested = [null, true, 42n, 1.5, "value"]
    const composite = Value.fromJS(nullPrototype)
    const compositeData = composite.toJS()
    assert.equal(Object.getPrototypeOf(compositeData), null)
    assert.equal(compositeData.__proto__, "data")
    assert.deepEqual(compositeData.nested, [null, true, 42n, 1.5, "value"])
    assert.ok(Object.isFrozen(compositeData))
    assert.ok(Object.isFrozen(compositeData.nested))
    assert.equal(composite.kind(), "map")
    assert.equal(composite.kind(["nested"]), "list")
    assert.equal(composite.kind(["nested", 2]), "int")
    assert.equal(composite.has(["nested", 9]), false)
    assert.equal(composite.has(["missing"]), false)
    assert.equal(composite.get(["nested", 2]), 42n)
    assert.throws(() => composite.get(["missing"]), error =>
      error instanceof CollaError && error.code === "missing_key" &&
      error.path[0] === "missing")

    const firstOrder = Value.fromJS({ "": 1, "𐀀": 2 })
    const secondOrderInput = Object.create(null)
    secondOrderInput["𐀀"] = 2
    secondOrderInput[""] = 1
    const secondOrder = Value.fromJS(secondOrderInput)
    assert.deepEqual(firstOrder.encode(), secondOrder.encode())

    assert.ok(Object.isFrozen(DEFAULT_INPUT_LIMITS))
    assert.equal(DEFAULT_INPUT_LIMITS.maxDepth, 128)
    assert.equal(DEFAULT_INPUT_LIMITS.maxStringBytes, 16 * 1024 * 1024)
    assert.throws(
      () => Value.fromJS("too long", { limits: { maxStringBytes: 2 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "string bytes" && error.details.maximum === 2,
    )
    const limitedBytes = Value.fromJS("abcd").encode()
    assert.throws(
      () => Value.decode(limitedBytes, { limits: { maxStringBytes: 3 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded",
    )

    const invalidValues = [
      NaN,
      Infinity,
      1n << 63n,
      new Date(),
      new Set(),
      new Map(),
      new (class Example {})(),
      [, 1],
    ]
    for (const invalid of invalidValues) {
      assert.throws(() => Value.fromJS(invalid), error =>
        error instanceof CollaError &&
        (error.code === "invalid_value" || error.code === "invalid_argument"))
    }
    const cyclic = {}
    cyclic.self = cyclic
    assert.throws(() => Value.fromJS(cyclic), error =>
      error instanceof CollaError && error.code === "invalid_value")
    const accessor = {}
    Object.defineProperty(accessor, "value", { get() { throw new Error("must not run") } })
    assert.throws(() => Value.fromJS(accessor), error =>
      error instanceof CollaError && error.code === "invalid_value")
    const symbolRecord = { [Symbol("x")]: 1 }
    assert.throws(() => Value.fromJS(symbolRecord), error =>
      error instanceof CollaError && error.code === "invalid_value")
    assert.throws(() => int(Number.MAX_SAFE_INTEGER + 1), error =>
      error instanceof CollaError && error.code === "invalid_argument")
    assert.throws(() => Value.fromJS(null, { limits: { unknown: 1 } }), error =>
      error instanceof CollaError && error.code === "invalid_argument")

    const base = Value.fromJS("draft", { limits: { maxStringBytes: 5 } })
    const integer = Value.fromJS(42n)
    assert.equal(integer.toJS(), 42n)
    const clone = base.clone()
    const baseBytes = base.encode()
    const decodedBase = Value.decode(baseBytes)
    assert.equal(decodedBase.toJS(), "draft")
    assert.notEqual(decodedBase.encode().buffer, baseBytes.buffer)

    const builder = base.change()
    base.dispose()
    const change = builder.replace([], "published value larger than receiver input policy").build()
    assert.throws(() => builder.build(), error =>
      error instanceof CollaError && error.code === "invalid_state" &&
      error.details.reason === "consumed")

    const changeBytes = change.encode()
    const decodedChange = Change.decode(changeBytes)
    const next = apply(clone, decodedChange)
    assert.equal(next.toJS(), "published value larger than receiver input policy")
    assert.equal(clone.toJS(), "draft")

    assert.throws(
      () => Change.decode(changeBytes, { limits: { maxChangeNodes: 0 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded",
    )

    assert.throws(() => Change.decode(Uint8Array.of(255)), error =>
      error instanceof CollaError && error.code === "invalid_encoding" &&
      error.operation === "change_decode" && Object.isFrozen(error.details))

    for (const handle of [
      composite,
      firstOrder,
      secondOrder,
      integer,
      clone,
      decodedBase,
      change,
      decodedChange,
      next,
    ]) {
      handle.dispose()
      handle.dispose()
    }
    assert.throws(() => next.toJS(), error =>
      error instanceof CollaError && error.code === "invalid_state" &&
      error.details.reason === "disposed")
  `
  await writeFile(join(fixtureDir, "fixture.mjs"), fixture)
  execFileSync(process.execPath, ["fixture.mjs"], { cwd: fixtureDir, stdio: "inherit" })
} finally {
  await rm(fixtureDir, { recursive: true, force: true })
}
