import assert from "node:assert/strict"
import { test } from "node:test"

import {
  buildListOps,
  buildTextOps,
  Change,
  CollaError,
  convertChangeToEditSteps,
  countCodePoints,
  inspectChange,
  richText,
  resolveCodePointPosition,
  text,
  ValueHandle,
} from "../dist/node.js"

function assertDeepFrozen(value) {
  if (value === null || typeof value !== "object") return
  assert.ok(Object.isFrozen(value))
  for (const child of Object.values(value)) assertDeepFrozen(child)
}

test("convertChangeToEditSteps preserves scalar and map change semantics", () => {
  const base = ValueHandle.fromJS({
    count: 1n,
    nested: { count: 2n, label: "old" },
    removed: "gone",
  })
  const change = Change.fromJS({
    type: "map",
    entries: [
      { key: "created", type: "insert", value: { ready: true } },
      {
        key: "nested",
        type: "modify",
        change: {
          type: "map",
          entries: [
            { key: "count", type: "modify", change: { type: "int", delta: 3n } },
            { key: "label", type: "modify", change: { type: "replace", value: "new" } },
          ],
        },
      },
      { key: "removed", type: "delete" },
    ],
  })

  const steps = convertChangeToEditSteps(change, base)
  assert.deepEqual(steps, [
    { type: "map", path: ["created"], op: { type: "insert", value: Object.assign(Object.create(null), { ready: true }) } },
    { type: "int", path: ["nested", "count"], delta: 3n },
    { type: "replace", path: ["nested", "label"], value: "new" },
    { type: "map", path: ["removed"], op: { type: "delete" } },
  ])
  assertDeepFrozen(steps)

  const replace = Change.fromJS({ type: "replace", value: [1n, { fresh: true }] })
  const replaceSteps = convertChangeToEditSteps(replace, base)
  assert.deepEqual(replaceSteps, [{ type: "replace", path: [], value: [1n, Object.assign(Object.create(null), { fresh: true })] }])
  assertDeepFrozen(replaceSteps)

  const integer = ValueHandle.fromJS(5n)
  const add = Change.fromJS({ type: "int", delta: -2n })
  assert.deepEqual(convertChangeToEditSteps(add, integer), [{ type: "int", path: [], delta: -2n }])

  const noop = Change.fromJS({ type: "noop" })
  const empty = convertChangeToEditSteps(noop, base)
  assert.deepEqual(empty, [])
  assert.ok(Object.isFrozen(empty))
})

test("convertChangeToEditSteps preserves list operation flow and element-relative modify steps", () => {
  const base = ValueHandle.fromJS([
    "a",
    { count: 1n, label: "old" },
    "c",
    "d",
  ])
  const change = Change.fromJS({
    type: "list",
    ops: [
      { type: "retain", length: 1 },
      { type: "insert", values: ["x", { inserted: true }] },
      {
        type: "modify",
        change: {
          type: "map",
          entries: [
            { key: "count", type: "modify", change: { type: "int", delta: 2n } },
            { key: "label", type: "modify", change: { type: "replace", value: "new" } },
          ],
        },
      },
      { type: "delete", length: 1 },
    ],
  })

  const steps = convertChangeToEditSteps(change, base)
  assert.deepEqual(steps, [{
    type: "list",
    path: [],
    ops: [
      { type: "retain", length: 1 },
      { type: "insert", values: ["x", Object.assign(Object.create(null), { inserted: true })] },
      {
        type: "modify",
        steps: [
          { type: "int", path: ["count"], delta: 2n },
          { type: "replace", path: ["label"], value: "new" },
        ],
      },
      { type: "delete", length: 1 },
    ],
  }])
  assertDeepFrozen(steps)
})

test("convertChangeToEditSteps projects Text and RichText consumption to UTF-16", () => {
  const textBase = ValueHandle.fromJS(text("😀A😃"))
  const textChange = Change.fromJS({
    type: "text",
    ops: [
      { type: "retain", length: 2 },
      { type: "insert", text: "X" },
      { type: "delete", length: 1 },
    ],
  })
  assert.deepEqual(convertChangeToEditSteps(textChange, textBase), [{
    type: "text",
    path: [],
    ops: [
      { type: "retain", length: 3 },
      { type: "insert", text: "X" },
      { type: "delete", length: 2 },
    ],
  }])

  const richBase = ValueHandle.fromJS(richText([
    { type: "text", text: "😀A", attrs: { bold: true } },
    { type: "embed", value: { id: "old" } },
    { type: "text", text: "😃B" },
  ]))
  const richChange = Change.fromJS({
    type: "richtext",
    ops: [
      { type: "retain", length: 1, patch: { bold: { type: "remove" }, count: { type: "set", value: 2n } } },
      { type: "insert", content: { type: "text", text: "X", attrs: { italic: true } } },
      { type: "delete", length: 1 },
      { type: "retain", length: 1 },
      { type: "insert", content: { type: "embed", value: { id: "new" }, attrs: { kind: "chip" } } },
      { type: "delete", length: 1 },
    ],
  })

  const steps = convertChangeToEditSteps(richChange, richBase)
  assert.deepEqual(steps, [{
    type: "richtext",
    path: [],
    ops: [
      {
        type: "retain",
        length: 2,
        patch: Object.assign(Object.create(null), {
          bold: { type: "remove" },
          count: { type: "set", value: 2n },
        }),
      },
      { type: "insert", span: { type: "text", text: "X", attrs: Object.assign(Object.create(null), { italic: true }) } },
      { type: "delete", length: 1 },
      { type: "retain", length: 1 },
      {
        type: "insert",
        span: {
          type: "embed",
          value: Object.assign(Object.create(null), { id: "new" }),
          attrs: Object.assign(Object.create(null), { kind: "chip" }),
        },
      },
      { type: "delete", length: 2 },
    ],
  }])
  assertDeepFrozen(steps)
})

test("convertChangeToEditSteps projects fragmented UTF-16 consumption across RichText spans", () => {
  const textBase = ValueHandle.fromJS(text("😀a😃b"))
  const textChange = Change.fromJS({
    type: "text",
    ops: [
      { type: "retain", length: 1 },
      { type: "delete", length: 1 },
      { type: "retain", length: 1 },
      { type: "delete", length: 1 },
    ],
  })
  assert.deepEqual(convertChangeToEditSteps(textChange, textBase), [{
    type: "text",
    path: [],
    ops: [
      { type: "retain", length: 2 },
      { type: "delete", length: 1 },
      { type: "retain", length: 2 },
      { type: "delete", length: 1 },
    ],
  }])

  const richBase = ValueHandle.fromJS(richText([
    { type: "text", text: "😀" },
    { type: "embed", value: { id: 1n } },
    { type: "text", text: "a😃" },
    { type: "embed", value: { id: 2n } },
    { type: "text", text: "b" },
  ]))
  const richChange = Change.fromJS({
    type: "richtext",
    ops: [
      { type: "retain", length: 1 },
      { type: "delete", length: 1 },
      { type: "retain", length: 1 },
      { type: "delete", length: 1 },
      { type: "insert", content: { type: "text", text: "X" } },
    ],
  })
  assert.deepEqual(convertChangeToEditSteps(richChange, richBase), [{
    type: "richtext",
    path: [],
    ops: [
      { type: "retain", length: 2 },
      { type: "delete", length: 1 },
      { type: "retain", length: 1 },
      { type: "insert", span: { type: "text", text: "X" } },
      { type: "delete", length: 2 },
    ],
  }])
})

test("convertChangeToEditSteps handles large fragmented operation streams", () => {
  const length = 20_000
  const textBase = ValueHandle.fromJS(text("a".repeat(length)))
  const textOps = []
  for (let index = 0; index < length; index += 1) {
    textOps.push({ type: "retain", length: 1 }, { type: "insert", text: "x" })
  }
  const textSteps = convertChangeToEditSteps(
    Change.fromJS({ type: "text", ops: textOps }),
    textBase,
  )
  assert.equal(textSteps[0].ops.length, length * 2)
  assert.deepEqual(textSteps[0].ops.at(-1), { type: "insert", text: "x" })

  const richBase = ValueHandle.fromJS(richText([{ type: "text", text: "a".repeat(length) }]))
  const richOps = []
  for (let index = 0; index < length; index += 1) {
    richOps.push(
      { type: "retain", length: 1 },
      { type: "insert", content: { type: "embed", value: null } },
    )
  }
  const richSteps = convertChangeToEditSteps(
    Change.fromJS({ type: "richtext", ops: richOps }),
    richBase,
  )
  assert.equal(richSteps[0].ops.length, length * 2)
  assert.deepEqual(richSteps[0].ops.at(-1), {
    type: "insert",
    span: { type: "embed", value: null },
  })
})

test("richtext is the only JavaScript-facing RichText discriminator", () => {
  const marker = richText([{ type: "text", text: "a" }])
  const value = ValueHandle.fromJS(marker)
  assert.equal(value.kind(), "richtext")
  assert.equal(value.toJS().type, "richtext")

  const change = Change.fromJS({
    type: "richtext",
    ops: [{ type: "insert", content: { type: "text", text: "x" } }],
  })
  assert.deepEqual(convertChangeToEditSteps(change, value)[0].type, "richtext")
  assert.equal(inspectChange(change, value)[0].type, "richtext.insertText")

  const incompatible = Change.fromJS({ type: "richtext", ops: [{ type: "delete", length: 1 }] })
  assert.throws(
    () => convertChangeToEditSteps(incompatible, ValueHandle.fromJS(text("a"))),
    error => error instanceof CollaError && error.operation === "convert_change_to_edit_steps" &&
      error.details.expected === "richtext" && error.details.actual === "text",
  )
  assert.throws(
    () => resolveCodePointPosition(value, [], 99),
    error => error instanceof CollaError && error.details.target === "richtext",
  )
})

test("buildTextOps accurately converts UTF-16 cursor with emojis to Unicode codepoints in single pass", () => {
  const base = "A😀B🎉C"
  assert.equal(countCodePoints(base, 0, 1), 1)
  assert.equal(countCodePoints(base, 1, 2), 1)
  assert.equal(countCodePoints(base, 0, 3), 2)

  const ops = buildTextOps(base, stream => {
    stream.retain(3)
    stream.insert("🚀")
    stream.retain(1)
    stream.delete(2)
  })

  assert.deepEqual(ops, [
    { type: "retain", length: 2 },
    { type: "insert", text: "🚀" },
    { type: "retain", length: 1 },
    { type: "delete", length: 1 },
  ])
})

test("CollaError has strongly typed details and is() type guard", () => {
  const err = new CollaError("type_mismatch", "apply", { expected: "text", actual: "map" })
  assert.equal(err.is("type_mismatch"), true)
  assert.equal(err.is("limit_exceeded"), false)
  assert.equal(err.code, "type_mismatch")
  assert.equal(err.operation, "apply")
  assert.equal(err.details.expected, "text")
  assert.equal(err.details.actual, "map")
  assert.ok(err.message.includes("apply failed: type_mismatch"))
})

