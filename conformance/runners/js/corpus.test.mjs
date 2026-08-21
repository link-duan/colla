// Conformance runner for the JavaScript implementation. Walks the same
// language-neutral corpus as the Rust reference runner and checks it against the
// public `colla-ot` API. Run with `node --test` after building the package.
//
// See `conformance/README.md` for the fixture format. RichText content is not
// part of the current seed corpus and its neutral conversion is deferred.

import assert from "node:assert/strict"
import { readdirSync, readFileSync, statSync } from "node:fs"
import { dirname, extname, join, relative } from "node:path"
import { test } from "node:test"
import { fileURLToPath } from "node:url"

import {
  apply,
  Change,
  CollaError,
  compose,
  invert,
  text,
  transformPair,
  Value,
} from "../../../packages/core/dist/node.js"

const here = dirname(fileURLToPath(import.meta.url))
const versionDir = join(here, "..", "..", "corpus", "v1")

// --- Neutral conversion -----------------------------------------------------

function singleTag(node, what) {
  if (typeof node !== "object" || node === null || Array.isArray(node)) {
    throw new Error(`${what} must be a single-key tagged object`)
  }
  const keys = Object.keys(node)
  if (keys.length !== 1) {
    throw new Error(`${what} object must have exactly one tag, found ${keys.length}`)
  }
  return [keys[0], node[keys[0]]]
}

function valueFromNeutral(node) {
  const [tag, body] = singleTag(node, "value")
  switch (tag) {
    case "null":
      return null
    case "bool":
      return body
    case "int":
      return BigInt(body)
    case "float":
      return body
    case "string":
      return body
    case "text":
      return text(body)
    case "list":
      return body.map(valueFromNeutral)
    case "map":
      return Object.fromEntries(
        Object.entries(body).map(([key, value]) => [key, valueFromNeutral(value)]),
      )
    case "richtext":
      throw new Error("`richtext` neutral conversion is not implemented yet")
    default:
      throw new Error(`unknown value tag: ${tag}`)
  }
}

function changeFromNeutral(node) {
  const [tag, body] = singleTag(node, "change")
  switch (tag) {
    case "noop":
      return { type: "noop" }
    case "replace":
      return { type: "replace", value: valueFromNeutral(body) }
    case "int":
      return { type: "int", delta: BigInt(body.add) }
    case "map":
      return {
        type: "map",
        entries: Object.entries(body).map(([key, entry]) => mapEntryFromNeutral(key, entry)),
      }
    case "list":
      return { type: "list", ops: body.map(listOpFromNeutral) }
    case "text":
      return { type: "text", ops: body.map(textOpFromNeutral) }
    case "richtext":
      throw new Error("`richtext` neutral conversion is not implemented yet")
    default:
      throw new Error(`unknown change tag: ${tag}`)
  }
}

function mapEntryFromNeutral(key, node) {
  const [tag, body] = singleTag(node, "map entry change")
  switch (tag) {
    case "insert":
      return { key, type: "insert", value: valueFromNeutral(body) }
    case "delete":
      return { key, type: "delete" }
    case "modify":
      return { key, type: "modify", change: changeFromNeutral(body) }
    default:
      throw new Error(`unknown map entry change tag: ${tag}`)
  }
}

function listOpFromNeutral(node) {
  const [tag, body] = singleTag(node, "list op")
  switch (tag) {
    case "retain":
      return { type: "retain", length: body }
    case "delete":
      return { type: "delete", length: body }
    case "insert":
      return { type: "insert", values: body.map(valueFromNeutral) }
    case "modify":
      return { type: "modify", change: changeFromNeutral(body) }
    default:
      throw new Error(`unknown list op tag: ${tag}`)
  }
}

function textOpFromNeutral(node) {
  const [tag, body] = singleTag(node, "text op")
  switch (tag) {
    case "retain":
      return { type: "retain", length: body }
    case "delete":
      return { type: "delete", length: body }
    case "insert":
      return { type: "insert", text: body }
    default:
      throw new Error(`unknown text op tag: ${tag}`)
  }
}

// --- Helpers ----------------------------------------------------------------

function toHex(bytes) {
  let out = ""
  for (const byte of bytes) out += byte.toString(16).padStart(2, "0")
  return out
}

function decodeHex(hex) {
  if (hex.length % 2 !== 0) throw new Error("hex string has an odd length")
  const bytes = new Uint8Array(hex.length / 2)
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16)
  }
  return bytes
}

function value(node) {
  return Value.fromJS(valueFromNeutral(node))
}

function change(node) {
  return Change.fromJS(changeFromNeutral(node))
}

function sameBytes(actual, expected, label) {
  assert.equal(toHex(actual), toHex(expected), label)
}

function assertErrorCode(fn, expected, label) {
  assert.throws(fn, (error) => {
    assert.ok(error instanceof CollaError, `${label}: expected a CollaError, got ${error}`)
    assert.equal(error.code, expected, `${label}: error code`)
    return true
  })
}

// --- Fixture kinds ----------------------------------------------------------

function runValueCodec(fx) {
  const built = value(fx.value)
  const expected = decodeHex(fx.canonicalBytes)
  sameBytes(built.encode(), expected, "encode(value) == canonicalBytes")
  const decoded = Value.decode(expected)
  sameBytes(decoded.encode(), expected, "decode round-trip is canonical")
  sameBytes(decoded.encode(), built.encode(), "decoded value equals the fixture value")
}

function runDecodeError(fx) {
  const bytes = decodeHex(fx.inputBytes)
  const decode =
    fx.target === "value"
      ? () => Value.decode(bytes)
      : fx.target === "change"
        ? () => Change.decode(bytes)
        : null
  if (decode === null) throw new Error(`unknown decode target: ${fx.target}`)
  assertErrorCode(decode, fx.expectError.code, "decode-error")
}

function runApply(fx) {
  const base = value(fx.snapshot)
  const edit = change(fx.change)
  if (typeof fx.changeBytes === "string") {
    sameBytes(edit.encode(), decodeHex(fx.changeBytes), "changeBytes")
  }
  if (fx.expectError) {
    assertErrorCode(() => apply(base, edit), fx.expectError.code, "apply")
    return
  }
  const result = apply(base, edit)
  sameBytes(result.encode(), value(fx.expect.value).encode(), "apply result")
}

function runCompose(fx) {
  const [first, second] = fx.changes.map(change)
  if (fx.expectError) {
    assertErrorCode(() => compose(first, second), fx.expectError.code, "compose")
    return
  }
  const composed = compose(first, second)
  sameBytes(composed.encode(), change(fx.expect.change).encode(), "composed change")
  if (fx.snapshot !== undefined) {
    const base = value(fx.snapshot)
    const viaComposed = apply(base, composed).encode()
    const sequential = apply(apply(base, first), second).encode()
    sameBytes(viaComposed, sequential, "compose convergence")
  }
}

function runInvert(fx) {
  const base = value(fx.snapshot)
  const edit = change(fx.change)
  const inverse = invert(edit, base)
  sameBytes(inverse.encode(), change(fx.expect.change).encode(), "inverse change")
  const restored = apply(apply(base, edit), inverse).encode()
  sameBytes(restored, base.encode(), "invert round-trip restores the snapshot")
}

function runTransform(fx) {
  const changeA = change(fx.changeA)
  const changeB = change(fx.changeB)
  const order = fx.side === "left" ? "left-first" : fx.side === "right" ? "right-first" : null
  if (order === null) throw new Error(`unknown tie-break side: ${fx.side}`)
  const [aPrime, bPrime] = transformPair(changeA, changeB, { order })
  sameBytes(aPrime.encode(), change(fx.expect.aPrime).encode(), "aPrime")
  sameBytes(bPrime.encode(), change(fx.expect.bPrime).encode(), "bPrime")
  if (fx.base !== undefined) {
    const base = value(fx.base)
    const leftPath = apply(apply(base, changeA), bPrime).encode()
    const rightPath = apply(apply(base, changeB), aPrime).encode()
    sameBytes(leftPath, rightPath, "transform convergence")
  }
}

const RUNNERS = {
  "value-codec": runValueCodec,
  "decode-error": runDecodeError,
  apply: runApply,
  compose: runCompose,
  invert: runInvert,
  transform: runTransform,
}

// --- Corpus walk ------------------------------------------------------------

function collectJson(dir) {
  const files = []
  for (const name of readdirSync(dir).sort()) {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) files.push(...collectJson(full))
    else if (extname(full) === ".json") files.push(full)
  }
  return files
}

function expectedId(path) {
  return relative(versionDir, path).replace(/\.json$/, "").replaceAll("\\", "/")
}

const files = collectJson(versionDir)
assert.ok(files.length > 0, `no fixtures found under ${versionDir}`)

for (const path of files) {
  const fx = JSON.parse(readFileSync(path, "utf8"))
  assert.equal(fx.id, expectedId(path), `${path}: id must equal the corpus-relative path`)
  assert.equal(fx.corpusVersion, 1, `${fx.id}: corpusVersion must be 1`)
  const runner = RUNNERS[fx.kind]
  if (runner === undefined) throw new Error(`${fx.id}: unknown fixture kind: ${fx.kind}`)
  test(fx.id, () => runner(fx))
}
