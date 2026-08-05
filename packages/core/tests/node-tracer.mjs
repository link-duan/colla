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
      resolveCodePointPosition,
      resolveUtf16Position,
      text,
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

    const atomicString = Value.fromJS("A😀B")
    const collaborativeText = Value.fromJS(text("A😀B"))
    assert.equal(atomicString.kind(), "string")
    assert.equal(collaborativeText.kind(), "text")
    assert.deepEqual(collaborativeText.toJS(), text("A😀B"))
    const decodedText = Value.decode(collaborativeText.encode())
    assert.equal(decodedText.kind(), "text")
    assert.deepEqual(
      collaborativeText.encode(),
      Uint8Array.from([6, 6, 65, 240, 159, 152, 128, 66]),
    )
    assert.equal(resolveCodePointPosition(collaborativeText, [], 0), 0)
    assert.equal(resolveCodePointPosition(collaborativeText, [], 1), 1)
    assert.equal(resolveCodePointPosition(collaborativeText, [], 3), 2)
    assert.equal(resolveCodePointPosition(collaborativeText, [], 4), 3)
    assert.equal(resolveUtf16Position(collaborativeText, [], 2), 3)
    assert.throws(
      () => resolveCodePointPosition(collaborativeText, [], 2),
      error => error instanceof CollaError && error.code === "invalid_utf16_boundary" &&
        error.details.position === 2,
    )
    assert.throws(() => Value.fromJS("\\ud800"), error =>
      error instanceof CollaError && error.code === "invalid_value")
    assert.throws(() => text("\\udc00"), error =>
      error instanceof CollaError && error.code === "invalid_value")
    assert.throws(
      () => Value.fromJS(text("ab"), { limits: { maxStringBytes: 1 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "text bytes",
    )

    const trustedTextBase = Value.fromJS(text("a"), { limits: { maxStringBytes: 1 } })
    const trustedTextChange = trustedTextBase.change()
      .text([], editor => editor.insert(1, " larger"))
      .build()
    const trustedTextNext = apply(trustedTextBase, trustedTextChange)
    assert.deepEqual(trustedTextNext.toJS(), text("a larger"))

    const textBase = Value.fromJS({ title: text("A😀B") })
    const textBuilder = textBase.change()
    let escapedText
    const textChange = textBuilder.text(["title"], editor => {
      escapedText = editor
      editor.insert(3, "X").delete({ from: 0, to: 1 }).replace({ from: 2, to: 3 }, "Y")
    }).build()
    assert.deepEqual(
      textChange.encode(),
      Uint8Array.from([2, 1, 5, 116, 105, 116, 108, 101, 2, 4, 3, 2, 1, 0, 1, 1, 1, 89]),
    )
    assert.throws(
      () => Change.decode(textChange.encode(), { limits: { maxSequenceLength: 1 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded",
    )
    assert.throws(() => escapedText.insert(0, "late"), error =>
      error instanceof CollaError && error.code === "invalid_state")
    const textNext = apply(textBase, textChange)
    assert.deepEqual(textNext.get(["title"]), text("😀YB"))

    const textRollback = textBase.change()
    assert.throws(
      () => textRollback.text(["title"], editor => {
        editor.insert(4, "temporary").delete({ from: 2, to: 3 })
      }),
      error => error instanceof CollaError && error.code === "invalid_utf16_boundary",
    )
    textRollback.text(["title"], editor => editor.insert(0, "").delete({ from: 1, to: 1 }))
    const textRollbackChange = textRollback.build()
    const textRollbackNext = apply(textBase, textRollbackChange)
    assert.deepEqual(textRollbackNext.toJS(), textBase.toJS())

    const structuredBase = Value.fromJS({
      meta: { status: "draft" },
      items: ["a", "b"],
    })
    let escapedMap
    const structuredBuilder = structuredBase.change()
      .map(["meta"], map => {
        escapedMap = map
        map.set("status", "published").set("owner", "team").delete("missing")
      })
      .list(["items"], list => {
        list.insert(1, ["x"]).set(2, "B").delete({ from: 0, to: 1 })
      })
    assert.throws(() => escapedMap.set("late", true), error =>
      error instanceof CollaError && error.code === "invalid_state" &&
      error.details.reason === "scope_closed")
    const structuredChange = structuredBuilder.build()
    assert.deepEqual(structuredChange.encode(), Uint8Array.from([
      2, 2, 5, 105, 116, 101, 109, 115, 2, 3, 3, 1, 1, 5, 1, 120, 2, 1, 3, 1, 5,
      1, 66, 4, 109, 101, 116, 97, 2, 2, 2, 5, 111, 119, 110, 101, 114, 0, 5, 4,
      116, 101, 97, 109, 6, 115, 116, 97, 116, 117, 115, 2, 1, 5, 9, 112, 117, 98,
      108, 105, 115, 104, 101, 100,
    ]))
    const structuredNext = apply(structuredBase, structuredChange)
    assert.deepEqual(structuredNext.toJS(), Object.assign(Object.create(null), {
      items: ["x", "B"],
      meta: Object.assign(Object.create(null), { owner: "team", status: "published" }),
    }))

    const rollbackBuilder = structuredBase.change()
    const callbackFailure = new Error("callback failure")
    assert.throws(
      () => rollbackBuilder.map(["meta"], map => {
        map.set("temporary", true)
        throw callbackFailure
      }),
      error => error === callbackFailure,
    )
    assert.throws(
      () => rollbackBuilder.map(["meta"], map => {
        map.set("temporary", true).set("invalid", new Date())
      }),
      error => error instanceof CollaError && error.code === "invalid_value",
    )
    assert.throws(
      () => rollbackBuilder.list(["items"], list => list.insert(9, ["x"])),
      error => error instanceof CollaError && error.code === "out_of_bounds",
    )
    rollbackBuilder
      .map(["meta"], map => map.set("status", "draft").delete("missing"))
      .list(["items"], list => list.insert(0, []).delete({ from: 1, to: 1 }))
    const rollbackChange = rollbackBuilder.build()
    const rollbackNext = apply(structuredBase, rollbackChange)
    assert.deepEqual(rollbackNext.toJS(), structuredBase.toJS())

    const asyncBuilder = structuredBase.change()
    assert.throws(
      () => asyncBuilder.map(["meta"], async map => map.set("async", true)),
      error => error instanceof CollaError && error.code === "invalid_argument",
    )
    const asyncChange = asyncBuilder.build()
    const asyncNext = apply(structuredBase, asyncChange)
    assert.deepEqual(asyncNext.toJS(), structuredBase.toJS())

    const independentOriginal = Value.fromJS({ meta: {} })
    const independentSnapshot = independentOriginal.clone()
    const independentBuilder = independentOriginal.change()
    independentOriginal.dispose()
    const independentChange = independentBuilder
      .map(["meta"], map => map.set("alive", true))
      .build()
    const independentNext = apply(independentSnapshot, independentChange)
    assert.equal(independentNext.get(["meta", "alive"]), true)

    const abandoned = structuredBase.change()
    abandoned.dispose()
    assert.throws(() => abandoned.build(), error =>
      error instanceof CollaError && error.code === "invalid_state" &&
      error.details.reason === "disposed")

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
      atomicString,
      collaborativeText,
      decodedText,
      textBase,
      trustedTextBase,
      trustedTextChange,
      trustedTextNext,
      textChange,
      textNext,
      textRollbackChange,
      textRollbackNext,
      structuredBase,
      structuredChange,
      structuredNext,
      rollbackChange,
      rollbackNext,
      asyncChange,
      asyncNext,
      independentSnapshot,
      independentChange,
      independentNext,
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
