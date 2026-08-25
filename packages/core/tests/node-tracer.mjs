import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join, resolve } from "node:path"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

const packageDir = resolve(fileURLToPath(new URL("..", import.meta.url)))

test("node ESM package integration", async () => {
const packageJson = JSON.parse(await readFile(join(packageDir, "package.json"), "utf8"))
const fixtureDir = await mkdtemp(join(tmpdir(), "colla-node-tracer-"))
const packageSpec = process.env.COLLA_PACKAGE_SPEC

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
  const installedPackage = JSON.parse(await readFile(
    join(fixtureDir, "node_modules/colla-ot/package.json"),
    "utf8",
  ))
  if (process.env.COLLA_EXPECTED_PACKAGE_VERSION !== undefined) {
    assert.equal(installedPackage.version, process.env.COLLA_EXPECTED_PACKAGE_VERSION)
  }

  const fixture = `
    import assert from "node:assert/strict"
    import {
      apply,
      Change,
      CollaError,
      compose,
      DEFAULT_INPUT_LIMITS,
      int,
      inspectChange,
      invert,
      richText,
      resolveCodePointPosition,
      resolveUtf16Position,
      text,
      transformPair,
      Value,
    } from "colla-ot"

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
    // Byte decoding no longer enforces a receiver budget; it just round-trips.
    const roundTripped = Value.decode(limitedBytes)
    assert.deepEqual(roundTripped.toJS(), "abcd")
    roundTripped.dispose()

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
      Uint8Array.from([5, 6, 65, 240, 159, 152, 128, 66]),
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
        error.details.limit === "string bytes",
    )

    const trustedTextBase = Value.fromJS(text("a"), { limits: { maxStringBytes: 1 } })
    const trustedTextChange = Change.build(change => {
      change.text(editor => editor.retain(1).insert(" larger"))
    })
    const trustedTextNext = apply(trustedTextBase, trustedTextChange)
    assert.deepEqual(trustedTextNext.toJS(), text("a larger"))

    const textBase = Value.fromJS({ title: text("A😀B") })
    let escapedText
    const textChange = Change.build(change => {
      change.map(map => {
        map.modify("title", title => {
          title.text(editor => {
            escapedText = editor
            editor.delete(1).retain(1).insert("Y")
          })
        })
      })
    })
    assert.deepEqual(
      textChange.encode(),
      Uint8Array.from([2, 1, 5, 116, 105, 116, 108, 101, 2, 4, 3, 2, 1, 0, 1, 1, 1, 89]),
    )
    const decodedTextChange = Change.decode(textChange.encode())
    assert.deepEqual(decodedTextChange.encode(), textChange.encode())
    decodedTextChange.dispose()
    assert.throws(() => escapedText.insert("late"), error =>
      error instanceof CollaError && error.code === "invalid_state")
    const textNext = apply(textBase, textChange)
    assert.deepEqual(textNext.get(["title"]), text("😀YB"))

    assert.throws(
      () => Change.build(change => {
        change.map(map => map.modify("title", title => {
          title.text(editor => editor.retain(4).insert("temporary"))
        }))
      }, { limits: { maxSequenceLength: 3 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded",
    )

    const richInput = richText([
      { type: "text", text: "A😀", attrs: { bold: true, count: 2n, opacity: 0.5, label: "base" } },
      { type: "text", text: "B", attrs: { label: "base", opacity: 0.5, count: 2n, bold: true } },
      { type: "embed", value: { id: "one" }, attrs: { kind: "mention", bold: true } },
      { type: "text", text: "C" },
      { type: "text", text: "" },
    ])
    assert.equal(richInput.spans.length, 3)
    assert.equal(richInput.spans[0].text, "A😀B")
    assert.ok(Object.isFrozen(richInput))
    assert.ok(Object.isFrozen(richInput.spans))
    assert.ok(Object.isFrozen(richInput.spans[0]))
    assert.ok(Object.isFrozen(richInput.spans[0].attrs))

    const richAccessorSpans = []
    Object.defineProperty(richAccessorSpans, "0", {
      get() { throw new Error("must not run") },
    })
    richAccessorSpans.length = 1
    assert.throws(() => richText(richAccessorSpans), error =>
      error instanceof CollaError && error.code === "invalid_value")
    const richExtraSpans = [{ type: "text", text: "a" }]
    richExtraSpans.extra = true
    assert.throws(() => richText(richExtraSpans), error =>
      error instanceof CollaError && error.code === "invalid_value")
    const richSymbolSpans = [{ type: "text", text: "a" }]
    richSymbolSpans[Symbol("x")] = true
    assert.throws(() => richText(richSymbolSpans), error =>
      error instanceof CollaError && error.code === "invalid_value")

    assert.throws(
      () => Value.fromJS(richText([
        { type: "text", text: "a", attrs: { first: true } },
        { type: "text", text: "b", attrs: { second: true } },
      ]), { limits: { maxContainerLength: 1 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "container length",
    )
    assert.throws(
      () => Value.fromJS(richText([
        { type: "text", text: "a", attrs: { first: true, second: true } },
      ]), { limits: { maxContainerLength: 1 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "container length",
    )
    assert.throws(
      () => Value.fromJS(richText([
        { type: "text", text: "too long" },
      ]), { limits: { maxStringBytes: 2 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "string bytes",
    )
    assert.throws(
      () => Value.fromJS(richText([
        { type: "text", text: "a", attrs: { label: "too long" } },
      ]), { limits: { maxStringBytes: 2 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "string bytes",
    )
    assert.throws(
      () => Value.fromJS(richText([
        { type: "embed", value: { nested: true } },
      ]), { limits: { maxDepth: 1 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "depth",
    )
    assert.throws(
      () => Value.fromJS(richText([
        { type: "text", text: "abcdef" },
      ]), { limits: { maxSequenceLength: 3 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "sequence length",
    )
    assert.throws(
      () => Value.fromJS(richText([
        { type: "embed", value: true },
      ]), { limits: { maxValueNodes: 1 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "value nodes",
    )

    const richBase = Value.fromJS(richInput)
    assert.equal(richBase.kind(), "richText")
    const richData = richBase.toJS()
    assert.equal(richData.spans[0].attrs.count, 2n)
    assert.equal(richData.spans[0].attrs.opacity, 0.5)
    assert.equal(richData.spans[1].value.id, "one")
    assert.ok(Object.isFrozen(richData.spans[1].value))
    const richValueGolden = Uint8Array.from([
      6, 3, 0, 6, 65, 240, 159, 152, 128, 66, 4, 4, 98, 111, 108, 100, 0, 1, 5, 99,
      111, 117, 110, 116, 1, 4, 5, 108, 97, 98, 101, 108, 3, 4, 98, 97, 115, 101, 7,
      111, 112, 97, 99, 105, 116, 121, 2, 0, 0, 0, 0, 0, 0, 224, 63, 1, 8, 1, 2, 105,
      100, 4, 3, 111, 110, 101, 2, 4, 98, 111, 108, 100, 0, 1, 4, 107, 105, 110, 100,
      3, 7, 109, 101, 110, 116, 105, 111, 110, 0, 1, 67, 0,
    ])
    assert.deepEqual(richBase.encode(), richValueGolden)
    const rustRichBase = Value.decode(richValueGolden)
    assert.deepEqual(rustRichBase.toJS(), richBase.toJS())
    assert.equal(resolveCodePointPosition(richBase, [], 4), 3)
    assert.equal(resolveCodePointPosition(richBase, [], 5), 4)
    assert.equal(resolveUtf16Position(richBase, [], 4), 5)
    assert.throws(() => resolveCodePointPosition(richBase, [], 2), error =>
      error instanceof CollaError && error.code === "invalid_utf16_boundary")

    let escapedPatch
    const richChange = Change.build(change => {
      change.richText(editor => {
        editor
          .retain(2)
          .insertText("X", { color: "red", italic: true })
          .delete(1)
          .retain(1, patch => {
            escapedPatch = patch
            patch.remove("bold").set("color", "red")
          })
          .insertEmbed({ id: "two" }, { color: "red", kind: "chip" })
      })
    })
    assert.deepEqual(richChange.encode(), Uint8Array.from([
      5, 5, 0, 2, 0, 1, 0, 1, 88, 2, 5, 99, 111, 108, 111, 114, 3, 3, 114, 101, 100,
      6, 105, 116, 97, 108, 105, 99, 0, 1, 2, 1, 0, 1, 2, 4, 98, 111, 108, 100, 1, 5,
      99, 111, 108, 111, 114, 0, 3, 3, 114, 101, 100, 1, 1, 8, 1, 2, 105, 100, 4, 3,
      116, 119, 111, 2, 5, 99, 111, 108, 111, 114, 3, 3, 114, 101, 100, 4, 107, 105,
      110, 100, 3, 4, 99, 104, 105, 112,
    ]))
    assert.throws(() => escapedPatch.set("late", true), error =>
      error instanceof CollaError && error.code === "invalid_state" &&
      error.details.reason === "scope_closed")
    const richNext = apply(richBase, richChange)
    const richNextData = richNext.toJS()
    assert.equal(richNextData.spans.filter(span => span.type === "embed").length, 2)
    assert.equal(richNextData.spans.at(-1).text, "C")
    for (const span of richNextData.spans.slice(1, 4)) {
      assert.equal(span.attrs.color, "red")
      assert.equal(span.attrs.bold, undefined)
    }

    const patchFailure = new Error("patch failure")
    assert.throws(
      () => Change.build(change => {
        change.richText(editor => editor.retain(1, patch => {
          patch.set("temporary", true)
          throw patchFailure
        }))
      }),
      error => error === patchFailure,
    )

    const trustedRichBase = Value.fromJS(
      richText([{ type: "text", text: "a" }]),
      { limits: { maxStringBytes: 1, maxContainerLength: 1, maxValueNodes: 1 } },
    )
    const trustedRichChange = Change.build(change => {
      change.richText(editor => editor
        .retain(1)
        .insertText(" larger")
        .insertEmbed({ nested: ["larger"] }, { description: "larger" }))
    })
    const trustedRichNext = apply(trustedRichBase, trustedRichChange)
    assert.equal(trustedRichNext.toJS().spans[0].text, "a larger")
    assert.deepEqual(trustedRichNext.toJS().spans[1].value.nested, ["larger"])

    const algebraBase = Value.fromJS({
      count: 5n,
      meta: { status: "draft" },
      items: ["a", "b"],
      title: text("ab"),
      rich: richText([{ type: "text", text: "ab" }]),
      replace: "old",
    })
    let escapedInt
    const algebraFirst = Change.build(change => {
      change.map(map => {
        map.modify("count", value => {
          escapedInt = value
          value.intAdd(2n)
        })
        map.modify("meta", value => value.map(meta => meta.insert("owner", "team")))
        map.modify("items", value => value.list(list => list.retain(1).insert(["x"])))
        map.modify("title", value => value.text(title => title.retain(1).insert("X")))
        map.modify("rich", value => value.richText(rich => rich
          .retain(1)
          .insertText("X", { bold: true })))
        map.modify("replace", value => value.replace("middle"))
      })
    })
    const algebraFirstBytes = algebraFirst.encode()
    assert.throws(() => escapedInt.intAdd(1n), error =>
      error instanceof CollaError && error.code === "invalid_state")
    const algebraMiddle = apply(algebraBase, algebraFirst)
    const algebraSecond = Change.build(change => {
      change.map(map => {
        map.modify("count", value => value.intAdd(3n))
        map.modify("meta", value => value.map(meta => meta.modify("status", status => status.replace("published"))))
        map.modify("items", value => value.list(list => list.delete(1)))
        map.modify("title", value => value.text(title => title.delete(1)))
        map.modify("rich", value => value.richText(rich => rich
          .retain(2, patch => patch.set("color", "red"))))
        map.modify("replace", value => value.replace("final"))
      })
    })
    const algebraCombined = compose(algebraFirst, algebraSecond)
    const combinedGolden = Uint8Array.from([
      2, 6, 5, 99, 111, 117, 110, 116, 2, 6, 0, 10, 5, 105, 116, 101, 109, 115, 2,
      3, 2, 1, 1, 4, 1, 120, 2, 1, 4, 109, 101, 116, 97, 2, 2, 2, 5, 111, 119, 110,
      101, 114, 0, 4, 4, 116, 101, 97, 109, 6, 115, 116, 97, 116, 117, 115, 2, 1, 4,
      9, 112, 117, 98, 108, 105, 115, 104, 101, 100, 7, 114, 101, 112, 108, 97, 99,
      101, 2, 1, 4, 5, 102, 105, 110, 97, 108, 4, 114, 105, 99, 104, 2, 5, 2, 0, 1,
      1, 5, 99, 111, 108, 111, 114, 0, 3, 3, 114, 101, 100, 1, 0, 1, 88, 2, 4, 98,
      111, 108, 100, 0, 1, 5, 99, 111, 108, 111, 114, 3, 3, 114, 101, 100, 5, 116,
      105, 116, 108, 101, 2, 4, 2, 1, 1, 88, 2, 1,
    ])
    assert.deepEqual(algebraCombined.encode(), combinedGolden)
    const rustCombined = Change.decode(combinedGolden)
    assert.deepEqual(rustCombined.encode(), algebraCombined.encode())
    const algebraFinal = apply(algebraBase, algebraCombined)
    const rustCombinedFinal = apply(algebraBase, rustCombined)
    assert.deepEqual(rustCombinedFinal.toJS(), algebraFinal.toJS())
    const algebraSequential = apply(algebraMiddle, algebraSecond)
    assert.deepEqual(algebraFinal.toJS(), algebraSequential.toJS())
    const algebraInverse = invert(algebraCombined, algebraBase)
    assert.deepEqual(algebraInverse.encode(), Uint8Array.from([2,6,5,99,111,117,110,116,2,6,0,9,5,105,116,101,109,115,2,3,2,1,1,4,1,97,2,1,4,109,101,116,97,2,2,2,5,111,119,110,101,114,1,6,115,116,97,116,117,115,2,1,4,5,100,114,97,102,116,7,114,101,112,108,97,99,101,2,1,4,3,111,108,100,4,114,105,99,104,2,5,2,0,1,1,5,99,111,108,111,114,1,2,1,5,116,105,116,108,101,2,4,2,1,1,97,2,1]))
    const algebraRestored = apply(algebraFinal, algebraInverse)
    assert.deepEqual(algebraRestored.toJS(), algebraBase.toJS())

    const algebraRight = Change.build(change => {
      change.map(map => {
        map.modify("count", value => value.intAdd(4n))
        map.modify("meta", value => value.map(meta => meta.insert("reviewer", "qa")))
        map.modify("items", value => value.list(list => list.retain(1).insert(["y"])))
        map.modify("title", value => value.text(title => title.retain(1).insert("Y")))
        map.modify("rich", value => value.richText(rich => rich
          .retain(1)
          .insertText("Y", { italic: true })))
        map.modify("replace", value => value.replace("right"))
      })
    })
    const algebraRightBytes = algebraRight.encode()
    const algebraPair = transformPair(algebraFirst, algebraRight, { order: "left-first" })
    assert.ok(Object.isFrozen(algebraPair))
    assert.deepEqual(algebraPair[0].encode(), Uint8Array.from([2,6,5,99,111,117,110,116,2,6,0,4,5,105,116,101,109,115,2,3,2,0,1,1,1,4,1,120,4,109,101,116,97,2,2,1,5,111,119,110,101,114,0,4,4,116,101,97,109,7,114,101,112,108,97,99,101,2,1,4,6,109,105,100,100,108,101,4,114,105,99,104,2,5,2,0,1,0,1,0,1,88,1,4,98,111,108,100,0,1,5,116,105,116,108,101,2,4,2,0,1,1,1,88]))
    assert.deepEqual(algebraPair[1].encode(), Uint8Array.from([2,5,5,99,111,117,110,116,2,6,0,8,5,105,116,101,109,115,2,3,2,0,2,1,1,4,1,121,4,109,101,116,97,2,2,1,8,114,101,118,105,101,119,101,114,0,4,2,113,97,4,114,105,99,104,2,5,2,0,2,0,1,0,1,89,1,6,105,116,97,108,105,99,0,1,5,116,105,116,108,101,2,4,2,0,2,1,1,89]))
    const algebraAfterFirst = apply(algebraBase, algebraFirst)
    const algebraAfterRight = apply(algebraBase, algebraRight)
    const algebraLeftThen = apply(algebraAfterFirst, algebraPair[1])
    const algebraRightThen = apply(algebraAfterRight, algebraPair[0])
    assert.deepEqual(algebraLeftThen.toJS(), algebraRightThen.toJS())
    const algebraRightFirstPair = transformPair(
      algebraFirst,
      algebraRight,
      { order: "right-first" },
    )
    const algebraRightFirstAfterFirst = apply(algebraBase, algebraFirst)
    const algebraRightFirstAfterRight = apply(algebraBase, algebraRight)
    const algebraRightFirstLeftThen = apply(
      algebraRightFirstAfterFirst,
      algebraRightFirstPair[1],
    )
    const algebraRightFirstRightThen = apply(
      algebraRightFirstAfterRight,
      algebraRightFirstPair[0],
    )
    assert.deepEqual(algebraRightFirstLeftThen.toJS(), algebraRightFirstRightThen.toJS())
    assert.equal(algebraRightFirstLeftThen.get(["replace"]), "right")
    assert.deepEqual(algebraFirst.encode(), algebraFirstBytes)
    assert.deepEqual(algebraRight.encode(), algebraRightBytes)

    assert.throws(() => transformPair(algebraFirst, algebraRight, {}), error =>
      error instanceof CollaError && error.code === "invalid_argument")
    assert.throws(() => transformPair(algebraFirst, algebraRight, { order: "unknown" }), error =>
      error instanceof CollaError && error.code === "invalid_argument")
    const incompatibleText = Change.build(change => {
      change.text(value => value.retain(1).insert("b"))
    })
    assert.throws(() => compose(algebraFirst, incompatibleText), error =>
      error instanceof CollaError && error.code === "incompatible_change" &&
      error.details.reason === "kind_mismatch")
    assert.throws(() => transformPair(algebraFirst, incompatibleText, { order: "left-first" }), error =>
      error instanceof CollaError && error.code === "incompatible_change" &&
      error.details.reason === "kind_mismatch")
    const incompatibleBase = Value.fromJS(null)
    assert.throws(() => invert(algebraFirst, incompatibleBase), error =>
      error instanceof CollaError && error.code === "type_mismatch")

    const maxInt = Value.fromJS((1n << 63n) - 1n)
    const overflowChange = Change.build(change => change.intAdd(1n))
    assert.throws(() => apply(maxInt, overflowChange), error =>
      error instanceof CollaError && error.code === "integer_overflow")
    assert.throws(() => Change.build(change => change.intAdd(1)), error =>
      error instanceof CollaError && error.code === "invalid_argument")
    const overflowNoop = Change.build(change => change.intAdd(0n))
    const overflowSame = apply(maxInt, overflowNoop)
    assert.deepEqual(overflowSame.toJS(), maxInt.toJS())
    const deltaBase = Value.fromJS(0n)
    const maxDelta = Change.build(change => change.intAdd((1n << 63n) - 1n))
    const oneDelta = Change.build(change => change.intAdd(1n))
    assert.throws(() => compose(maxDelta, oneDelta), error =>
      error instanceof CollaError && error.code === "integer_overflow")

    const encodeVarint = value => {
      const bytes = []
      while (value >= 0x80n) {
        bytes.push(Number(value & 0x7fn) | 0x80)
        value >>= 7n
      }
      bytes.push(Number(value))
      return bytes
    }
    const logicalLength = 10_000_000n
    const largeDelete = Change.decode(
      Uint8Array.from([4, 1, 2, ...encodeVarint(logicalLength)]),
      { limits: { maxSequenceLength: Number(logicalLength) } },
    )
    const largeCombined = compose(largeDelete, largeDelete)
    assert.ok(largeCombined.encode().length < 16)

    const inspectBase = Value.fromJS({
      count: 5n,
      meta: { status: "draft", remove: "x" },
      items: ["a", "b", "c"],
      title: text("A😀B"),
      rich: richText([
        { type: "text", text: "A😀", attrs: { bold: true } },
        { type: "embed", value: { id: "old" } },
        { type: "text", text: "B" },
      ]),
      replace: "old",
    })
    const inspectedChange = Change.build(change => {
      change.map(map => {
        map.modify("count", value => value.intAdd(2n))
        map.modify("meta", value => value.map(meta => {
          meta.modify("status", status => status.replace("new")).delete("remove")
        }))
        map.modify("items", value => value.list(list => list
          .retain(1)
          .insert(["x"])
          .modify(item => item.replace("B"))
          .delete(1)))
        map.modify("title", value => value.text(title => title
          .retain(1)
          .delete(1)
          .insert("X")))
        map.modify("rich", value => value.richText(rich => rich
          .retain(2, patch => patch
            .remove("bold")
            .set("count", 2n)
            .set("opacity", 0.5)
            .set("label", "red"))
          .insertEmbed({ id: "new" }, { kind: "chip" })
          .delete(1)))
        map.modify("replace", value => value.replace("new"))
      })
    })
    const view = inspectChange(inspectedChange, inspectBase)
    assert.ok(Object.isFrozen(view))
    assert.deepEqual(view.map(entry => entry.type), [
      "int.add",
      "list.insert",
      "list.set",
      "list.delete",
      "map.delete",
      "map.set",
      "map.set",
      "richText.format",
      "richText.insertEmbed",
      "richText.delete",
      "text.insert",
      "text.delete",
    ])
    assert.deepEqual(view[1], {
      type: "list.insert",
      path: ["items"],
      index: 1,
      values: ["x"],
    })
    assert.equal(view[2].index, 1)
    assert.deepEqual(view[3].range, { from: 2, to: 3 })
    assert.deepEqual(view[9].range, { from: 3, to: 4 })
    assert.equal(view[10].at, 1)
    assert.deepEqual(view[11].range, { from: 1, to: 3 })
    assert.deepEqual(Object.keys(view[7].patch), ["bold", "count", "label", "opacity"])
    assert.deepEqual(view[7].patch.bold, { type: "remove" })
    assert.deepEqual(view[7].patch.count, { type: "set", value: 2n })
    assert.equal(view[8].attrs.kind, "chip")
    assert.equal(view[8].attrs.count, undefined)
    assert.ok(Object.isFrozen(view[7].path))
    assert.ok(Object.isFrozen(view[7].range))
    assert.ok(Object.isFrozen(view[7].patch))
    assert.ok(Object.isFrozen(view[7].patch.count))
    assert.ok(Object.isFrozen(view[8].embed))
    assert.ok(Object.isFrozen(view[8].attrs))

    const rootReplace = Change.build(change => change.replace({ fresh: true }))
    const rootReplaceView = inspectChange(rootReplace, inspectBase)
    assert.deepEqual(rootReplaceView, [{
      type: "value.replace",
      path: [],
      value: Object.assign(Object.create(null), { fresh: true }),
    }])
    const richPositionChange = Change.build(change => {
      change.map(map => map.modify("rich", value => value.richText(rich => rich
        .retain(2)
        .insertText("X")
        .delete(1))))
    })
    const richPositionView = inspectChange(richPositionChange, inspectBase)
    assert.equal(richPositionView[0].type, "richText.insertText")
    assert.equal(richPositionView[0].at, 3)
    assert.equal(richPositionView[0].attrs, undefined)
    assert.deepEqual(richPositionView[1].range, { from: 3, to: 4 })
    const noopChange = Change.build(change => change.noop())
    const noopView = inspectChange(noopChange, inspectBase)
    assert.deepEqual(noopView, [])
    assert.ok(Object.isFrozen(noopView))
    assert.throws(() => inspectChange(inspectedChange, incompatibleBase), error =>
      error instanceof CollaError && error.code === "type_mismatch" &&
      error.operation === "inspect_change")
    assert.throws(() => Change.decode(view), error =>
      error instanceof CollaError && error.code === "invalid_argument")

    const structuredBase = Value.fromJS({
      meta: { status: "draft" },
      items: ["a", "b"],
    })
    let escapedMap
    const structuredChange = Change.build(change => {
      change.map(map => {
        escapedMap = map
        map.modify("meta", meta => meta.map(value => value
          .modify("status", status => status.replace("published"))
          .insert("owner", "team")))
        map.modify("items", items => items.list(list => list
          .insert(["x"])
          .delete(1)
          .modify(item => item.replace("B"))))
      })
    })
    assert.throws(() => escapedMap.insert("late", true), error =>
      error instanceof CollaError && error.code === "invalid_state" &&
      error.details.reason === "scope_closed")
    assert.deepEqual(structuredChange.encode(), Uint8Array.from([2,2,5,105,116,101,109,115,2,3,3,1,1,4,1,120,2,1,3,1,4,1,66,4,109,101,116,97,2,2,2,5,111,119,110,101,114,0,4,4,116,101,97,109,6,115,116,97,116,117,115,2,1,4,9,112,117,98,108,105,115,104,101,100]))
    const structuredNext = apply(structuredBase, structuredChange)
    assert.deepEqual(structuredNext.toJS(), Object.assign(Object.create(null), {
      items: ["x", "B"],
      meta: Object.assign(Object.create(null), { owner: "team", status: "published" }),
    }))

    const callbackFailure = new Error("callback failure")
    assert.throws(
      () => Change.build(change => change.map(map => {
        map.insert("temporary", true)
        throw callbackFailure
      })),
      error => error === callbackFailure,
    )
    assert.throws(
      () => Change.build(() => {}),
      error => error instanceof CollaError && error.code === "invalid_argument" &&
        error.details.reason === "missing_change_kind",
    )
    assert.throws(
      () => Change.build(change => change.noop().replace(null)),
      error => error instanceof CollaError && error.code === "invalid_argument" &&
        error.details.reason === "duplicate_change_kind",
    )
    assert.throws(
      () => Change.build(async change => change.noop()),
      error => error instanceof CollaError && error.code === "invalid_argument",
    )
    assert.throws(
      () => Change.fromJS({
        type: "map",
        entries: [
          { key: "duplicate", type: "insert", value: 1 },
          { key: "duplicate", type: "delete" },
        ],
      }),
      error => error instanceof CollaError && error.code === "invalid_value",
    )
    assert.throws(
      () => Change.fromJS({
        type: "text",
        ops: [{ type: "retain", length: 4 }],
      }, { limits: { maxSequenceLength: 3 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded",
    )

    const assertBuilderEquivalent = (input, edit) => {
      const typed = Change.fromJS(input)
      const built = Change.build(edit)
      assert.deepEqual(built.encode(), typed.encode())
      typed.dispose()
      built.dispose()
    }
    assertBuilderEquivalent({ type: "noop" }, change => change.noop())
    assertBuilderEquivalent(
      { type: "replace", value: { nested: [true, 1n] } },
      change => change.replace({ nested: [true, 1n] }),
    )
    assertBuilderEquivalent({ type: "int", delta: -2n }, change => change.intAdd(-2n))
    assertBuilderEquivalent({
      type: "list",
      ops: [
        { type: "retain", length: 1 },
        { type: "insert", values: ["x", 2n] },
        { type: "modify", change: { type: "replace", value: "y" } },
        { type: "delete", length: 1 },
      ],
    }, change => change.list(list => list
      .retain(1)
      .insert(["x", 2n])
      .modify(item => item.replace("y"))
      .delete(1)))
    assertBuilderEquivalent({
      type: "text",
      ops: [
        { type: "retain", length: 2 },
        { type: "insert", text: "X" },
        { type: "delete", length: 1 },
      ],
    }, change => change.text(value => value.retain(2).insert("X").delete(1)))
    assertBuilderEquivalent({
      type: "richText",
      ops: [
        {
          type: "retain",
          length: 2,
          patch: {
            bold: { type: "remove" },
            color: { type: "set", value: "red" },
          },
        },
        { type: "insert", content: { type: "text", text: "X", attrs: { bold: true } } },
        { type: "insert", content: { type: "embed", value: { id: 1n }, attrs: { kind: "chip" } } },
        { type: "delete", length: 1 },
      ],
    }, change => change.richText(value => value
      .retain(2, patch => patch.remove("bold").set("color", "red"))
      .insertText("X", { bold: true })
      .insertEmbed({ id: 1n }, { kind: "chip" })
      .delete(1)))
    assertBuilderEquivalent({
      type: "map",
      entries: [
        { key: "created", type: "insert", value: true },
        { key: "removed", type: "delete" },
        {
          key: "nested",
          type: "modify",
          change: { type: "map", entries: [{ key: "value", type: "delete" }] },
        },
      ],
    }, change => change.map(map => map
      .insert("created", true)
      .delete("removed")
      .modify("nested", nested => nested.map(value => value.delete("value")))))

    const scalarBase = Value.fromJS(text("A😀B"))
    const scalarChange = Change.fromJS({
      type: "text",
      ops: [{ type: "retain", length: 2 }, { type: "insert", text: "X" }],
    })
    assert.deepEqual(apply(scalarBase, scalarChange).toJS(), text("A😀XB"))

    assert.throws(
      () => Change.build(change => change.map(map => map.modify("child", () => {}))),
      error => error instanceof CollaError && error.code === "invalid_argument" &&
        error.details.reason === "missing_change_kind",
    )
    assert.throws(
      () => Change.build(change => change.list(list => list.modify(child => child.noop().replace(null)))),
      error => error instanceof CollaError && error.code === "invalid_argument" &&
        error.details.reason === "duplicate_change_kind",
    )
    let escapedList
    Change.build(change => change.list(list => { escapedList = list }))
    assert.throws(() => escapedList.retain(1), error =>
      error instanceof CollaError && error.code === "invalid_state" &&
      error.details.reason === "scope_closed")
    let duplicateCallbackRan = false
    assert.throws(
      () => Change.build(change => {
        change.noop()
        change.map(() => { duplicateCallbackRan = true })
      }),
      error => error instanceof CollaError && error.details.reason === "duplicate_change_kind",
    )
    assert.equal(duplicateCallbackRan, false)

    for (const length of [-1, Number.MAX_SAFE_INTEGER + 1, 1.5]) {
      assert.throws(
        () => Change.fromJS({ type: "text", ops: [{ type: "retain", length }] }),
        error => error instanceof CollaError && error.code === "invalid_argument",
      )
    }
    assert.throws(
      () => Change.fromJS({ type: "text", ops: [{ type: "insert", text: "\\ud800" }] }),
      error => error instanceof CollaError && error.code === "invalid_value",
    )
    assert.throws(
      () => Change.fromJS({
        type: "richText",
        ops: [{ type: "insert", content: { type: "text", text: "x", value: null } }],
      }),
      error => error instanceof CollaError && error.code === "invalid_argument",
    )
    const cyclicChange = { type: "map", entries: [] }
    cyclicChange.entries.push({ key: "self", type: "modify", change: cyclicChange })
    assert.throws(() => Change.fromJS(cyclicChange), error =>
      error instanceof CollaError && error.code === "invalid_argument")
    assert.throws(
      () => Change.fromJS({
        type: "text",
        ops: [{ type: "retain", length: 0 }, { type: "delete", length: 0 }],
      }, { limits: { maxSequenceOps: 1 } }),
      error => error instanceof CollaError && error.code === "limit_exceeded" &&
        error.details.limit === "sequence ops",
    )
    const modifyNoop = Change.fromJS({
      type: "list",
      ops: [{ type: "modify", change: { type: "noop" } }],
    })
    assert.deepEqual(modifyNoop.encode(), Uint8Array.of(0))

    const base = Value.fromJS("draft", { limits: { maxStringBytes: 5 } })
    const integer = Value.fromJS(42n)
    assert.equal(integer.toJS(), 42n)
    const clone = base.clone()
    const baseBytes = base.encode()
    const decodedBase = Value.decode(baseBytes)
    assert.equal(decodedBase.toJS(), "draft")
    assert.notEqual(decodedBase.encode().buffer, baseBytes.buffer)

    base.dispose()
    const change = Change.build(value => value.replace("published value larger than receiver input policy"))

    const changeBytes = change.encode()
    const decodedChange = Change.decode(changeBytes)
    const next = apply(clone, decodedChange)
    assert.equal(next.toJS(), "published value larger than receiver input policy")
    assert.equal(clone.toJS(), "draft")
    const independentChangeClone = decodedChange.clone()
    decodedChange.dispose()
    const cloneNext = apply(clone, independentChangeClone)
    assert.equal(cloneNext.toJS(), "published value larger than receiver input policy")

    const decodedChangeAgain = Change.decode(changeBytes)
    assert.deepEqual(decodedChangeAgain.encode(), changeBytes)
    decodedChangeAgain.dispose()

    assert.throws(() => Change.decode(Uint8Array.of(255)), error =>
      error instanceof CollaError && error.code === "invalid_encoding" &&
      error.operation === "change_decode" && Object.isFrozen(error.details))
    for (const malformed of [
      Uint8Array.of(255),
      Uint8Array.of(5, 1, 255),
      Uint8Array.of(0, 0),
      Uint8Array.of(8, 128, 0),
      Uint8Array.of(4, 0, 0, 0, 0, 0, 0, 0, 128),
    ]) {
      assert.throws(() => Value.decode(malformed), error =>
        error instanceof CollaError && error.code === "invalid_encoding" &&
        error.operation === "value_decode")
    }
    // A MapChange whose keys are not strictly increasing is still rejected
    // (byte-canonical map ordering is enforced by cocodec).
    assert.throws(() => Change.decode(Uint8Array.of(2, 2, 1, 98, 1, 1, 97, 1)), error =>
      error instanceof CollaError && error.code === "invalid_encoding" &&
      error.operation === "change_decode")

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
      richBase,
      rustRichBase,
      richChange,
      richNext,
      trustedRichBase,
      trustedRichChange,
      trustedRichNext,
      algebraBase,
      algebraFirst,
      algebraMiddle,
      algebraSecond,
      algebraCombined,
      rustCombined,
      algebraFinal,
      rustCombinedFinal,
      algebraSequential,
      algebraInverse,
      algebraRestored,
      algebraRight,
      ...algebraPair,
      ...algebraRightFirstPair,
      algebraAfterFirst,
      algebraAfterRight,
      algebraLeftThen,
      algebraRightThen,
      algebraRightFirstAfterFirst,
      algebraRightFirstAfterRight,
      algebraRightFirstLeftThen,
      algebraRightFirstRightThen,
      incompatibleText,
      incompatibleBase,
      maxInt,
      overflowChange,
      overflowNoop,
      overflowSame,
      deltaBase,
      maxDelta,
      oneDelta,
      largeDelete,
      largeCombined,
      inspectBase,
      inspectedChange,
      rootReplace,
      richPositionChange,
      noopChange,
      structuredBase,
      structuredChange,
      structuredNext,
      integer,
      clone,
      decodedBase,
      change,
      decodedChange,
      independentChangeClone,
      cloneNext,
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
})
