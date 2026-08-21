import {
  applyHandles,
  ChangeHandle,
  composeHandles,
  inspectChangeHandle,
  invertHandle,
  resolveCodePointPositionHandle,
  resolveUtf16PositionHandle,
  transformPairHandles,
  ValueHandle,
} from "./internal/colla_wasm.js"

export type Path = readonly (string | number)[]
export type ValueKind =
  | "null"
  | "bool"
  | "int"
  | "float"
  | "string"
  | "text"
  | "richText"
  | "list"
  | "map"

export type ValueInput =
  | null
  | boolean
  | bigint
  | number
  | string
  | TextInput
  | RichTextInput
  | readonly ValueInput[]
  | ValueInputMap
export interface ValueInputMap {
  readonly [key: string]: ValueInput
}
export interface TextData {
  readonly type: "text"
  readonly value: string
}
export type TextInput = TextData
export type AttrValueData = boolean | bigint | number | string
export interface AttrsData {
  readonly [key: string]: AttrValueData
}
export type RichTextSpanData =
  | { readonly type: "text"; readonly text: string; readonly attrs?: AttrsData }
  | { readonly type: "embed"; readonly value: ValueData; readonly attrs?: AttrsData }
export type RichTextSpanInput =
  | { readonly type: "text"; readonly text: string; readonly attrs?: AttrsData }
  | { readonly type: "embed"; readonly value: ValueInput; readonly attrs?: AttrsData }
export interface RichTextData {
  readonly type: "richText"
  readonly spans: readonly RichTextSpanData[]
}
export interface RichTextInput {
  readonly type: "richText"
  readonly spans: readonly RichTextSpanInput[]
}
export type ValueData =
  | null
  | boolean
  | bigint
  | number
  | string
  | TextData
  | RichTextData
  | readonly ValueData[]
  | ValueDataMap
export interface ValueDataMap {
  readonly [key: string]: ValueData
}

export type ChangeInput =
  | { readonly type: "noop" }
  | { readonly type: "replace"; readonly value: ValueInput }
  | { readonly type: "map"; readonly entries: readonly MapChangeEntryInput[] }
  | { readonly type: "list"; readonly ops: readonly ListChangeOpInput[] }
  | { readonly type: "text"; readonly ops: readonly TextChangeOpInput[] }
  | { readonly type: "richText"; readonly ops: readonly RichTextChangeOpInput[] }
  | { readonly type: "int"; readonly delta: bigint }

export type MapChangeEntryInput =
  | { readonly key: string; readonly type: "insert"; readonly value: ValueInput }
  | { readonly key: string; readonly type: "delete" }
  | { readonly key: string; readonly type: "modify"; readonly change: ChangeInput }

export type ListChangeOpInput =
  | { readonly type: "retain"; readonly length: number }
  | { readonly type: "insert"; readonly values: readonly ValueInput[] }
  | { readonly type: "delete"; readonly length: number }
  | { readonly type: "modify"; readonly change: ChangeInput }

export type TextChangeOpInput =
  | { readonly type: "retain"; readonly length: number }
  | { readonly type: "insert"; readonly text: string }
  | { readonly type: "delete"; readonly length: number }

export type AttrPatchInput = Readonly<Record<
  string,
  | { readonly type: "set"; readonly value: AttrValueData }
  | { readonly type: "remove" }
>>

export type RichTextChangeOpInput =
  | { readonly type: "retain"; readonly length: number; readonly patch?: AttrPatchInput }
  | { readonly type: "insert"; readonly content: RichTextSpanInput }
  | { readonly type: "delete"; readonly length: number }

export interface InputLimits {
  readonly maxDepth: number
  readonly maxValueNodes: number
  readonly maxChangeNodes: number
  readonly maxContainerLength: number
  readonly maxStringBytes: number
  readonly maxSequenceOps: number
  readonly maxSequenceLength: number
}

export interface InputOptions {
  readonly limits?: Partial<InputLimits>
}

export const DEFAULT_INPUT_LIMITS: Readonly<InputLimits> = Object.freeze({
  maxDepth: 128,
  maxValueNodes: 1_000_000,
  maxChangeNodes: 1_000_000,
  maxContainerLength: 1_000_000,
  maxStringBytes: 16 * 1024 * 1024,
  maxSequenceOps: 1_000_000,
  maxSequenceLength: 1_000_000,
})

type ErrorPayload = {
  code?: string
  operation?: string
  details?: unknown
}

/**
 * Stable, cross-implementation error classification.
 *
 * The first group mirrors the `colla` core crate's `ErrorCode` (the single source
 * of truth, asserted by the conformance corpus). The trailing codes are produced
 * only by this JavaScript facade: `invalid_state` for operations on a disposed or
 * consumed handle, `invalid_argument` for malformed JavaScript input, and
 * `invalid_utf16_boundary` for UTF-16 position conversions. Maintained by hand to
 * match the core classification; see docs/adr/0015-error-code-classification.md.
 */
export type ErrorCode =
  | "invalid_encoding"
  | "limit_exceeded"
  | "type_mismatch"
  | "missing_key"
  | "key_already_exists"
  | "out_of_bounds"
  | "integer_overflow"
  | "incompatible_change"
  | "invalid_value"
  | "invalid_state"
  | "invalid_argument"
  | "invalid_utf16_boundary"

function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === "object" && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) deepFreeze(child)
    Object.freeze(value)
  }
  return value
}

function freezeDetails(value: unknown): Readonly<Record<string, unknown>> {
  if (value === null || typeof value !== "object") {
    return Object.freeze({ reason: String(value ?? "unknown error") })
  }
  return deepFreeze({ ...(value as Record<string, unknown>) })
}

export class CollaError extends Error {
  readonly code: ErrorCode
  readonly operation: string
  readonly path?: Path
  readonly details: Readonly<Record<string, unknown>>

  constructor(
    code: ErrorCode,
    operation: string,
    details: unknown,
    path?: Path,
  ) {
    super(`${operation} failed: ${code}`)
    this.name = "CollaError"
    this.code = code
    this.operation = operation
    this.path = path === undefined ? undefined : Object.freeze([...path])
    this.details = freezeDetails(details)
  }

  is(code: ErrorCode): boolean {
    return this.code === code
  }
}

function invalidArgument(operation: string, argument: string, reason: string): CollaError {
  return new CollaError("invalid_argument", operation, { argument, reason })
}

function invalidState(
  operation: string,
  resource: string,
  reason: "disposed" | "consumed" | "scope_closed",
): CollaError {
  return new CollaError("invalid_state", operation, { resource, reason })
}

function fromWasmError(error: unknown, fallbackOperation: string, path?: Path): CollaError {
  if (error instanceof CollaError) return error
  let payload: ErrorPayload = {}
  try {
    payload = JSON.parse(String(error)) as ErrorPayload
  } catch {
    payload = {}
  }
  return new CollaError(
    (payload.code ?? "invalid_argument") as ErrorCode,
    payload.operation ?? fallbackOperation,
    payload.details ?? { reason: String(error) },
    path,
  )
}

const valueFinalizer = new FinalizationRegistry<ValueHandle>(handle => handle.free())
const changeFinalizer = new FinalizationRegistry<ChangeHandle>(handle => handle.free())

const I64_MIN = -(1n << 63n)
const I64_MAX = (1n << 63n) - 1n

export function int(value: number | bigint): bigint {
  if (typeof value === "number" && !Number.isSafeInteger(value)) {
    throw invalidArgument("int", "value", "expected a safe integer")
  }
  if (typeof value !== "number" && typeof value !== "bigint") {
    throw invalidArgument("int", "value", "expected a number or bigint")
  }
  const result = BigInt(value)
  if (result < I64_MIN || result > I64_MAX) {
    throw invalidArgument("int", "value", "outside the signed 64-bit range")
  }
  return result
}

function assertWellFormedString(value: string, operation: string): string {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1)
      if (!(next >= 0xdc00 && next <= 0xdfff)) {
        throw new CollaError("invalid_value", operation, { reason: "unpaired UTF-16 surrogate" })
      }
      index += 1
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      throw new CollaError("invalid_value", operation, { reason: "unpaired UTF-16 surrogate" })
    }
  }
  return value
}

export function text(value: string): TextInput {
  if (typeof value !== "string") throw invalidArgument("text", "value", "expected a string")
  return Object.freeze({ type: "text", value: assertWellFormedString(value, "text") })
}

type AttrEntry = {
  readonly key: string
  readonly kind: "bool" | "int" | "float" | "string"
  readonly value: boolean | string | number
}

function attrEntries(
  attrs: AttrsData | undefined,
  operation: string,
  limits?: InputLimits,
): AttrEntry[] {
  if (attrs === undefined) return []
  if (!isRecord(attrs)) throw new CollaError("invalid_value", operation, { reason: "attributes must be a plain record" })
  const entries = ownDataEntries(attrs, operation)
  if (limits !== undefined && entries.length > limits.maxContainerLength) {
    limitExceeded("container length", entries.length, limits.maxContainerLength, operation)
  }
  const result: AttrEntry[] = []
  for (const [key, value] of entries) {
    assertWellFormedString(key, operation)
    const keyBytes = utf8.encode(key).length
    if (limits !== undefined && keyBytes > limits.maxStringBytes) {
      limitExceeded("string bytes", keyBytes, limits.maxStringBytes, operation)
    }
    if (typeof value === "boolean") {
      result.push({ key, kind: "bool", value })
    } else if (typeof value === "bigint") {
      if (value < I64_MIN || value > I64_MAX) {
        throw new CollaError("invalid_value", operation, { reason: "attribute Int is out of range" })
      }
      result.push({ key, kind: "int", value: value.toString() })
    } else if (typeof value === "number") {
      if (!Number.isFinite(value)) {
        throw new CollaError("invalid_value", operation, { reason: "attribute Float must be finite" })
      }
      result.push({ key, kind: "float", value: Object.is(value, -0) ? 0 : value })
    } else if (typeof value === "string") {
      assertWellFormedString(value, operation)
      const valueBytes = utf8.encode(value).length
      if (limits !== undefined && valueBytes > limits.maxStringBytes) {
        limitExceeded("string bytes", valueBytes, limits.maxStringBytes, operation)
      }
      result.push({ key, kind: "string", value })
    } else {
      throw new CollaError("invalid_value", operation, { reason: "unsupported attribute value" })
    }
  }
  return result.sort((left, right) => compareUtf8(left.key, right.key))
}

function attrsData(entries: readonly AttrEntry[]): AttrsData | undefined {
  if (entries.length === 0) return undefined
  const result = Object.create(null) as Record<string, AttrValueData>
  for (const entry of entries) {
    result[entry.key] = entry.kind === "int" ? BigInt(entry.value) : entry.value
  }
  return Object.freeze(result)
}

function sameAttrs(left: readonly AttrEntry[], right: readonly AttrEntry[]): boolean {
  return JSON.stringify(left) === JSON.stringify(right)
}

function writeAttrs(writer: ByteWriter, entries: readonly AttrEntry[]): void {
  writer.varint(BigInt(entries.length))
  for (const entry of entries) {
    writer.string(entry.key)
    writeAttrValue(writer, entry)
  }
}

function writeAttrValue(writer: ByteWriter, entry: AttrEntry): void {
  switch (entry.kind) {
    case "bool": writer.byte(entry.value ? 1 : 0); break
    case "int": writer.byte(2); writer.int64(BigInt(entry.value)); break
    case "float": writer.byte(3); writer.float64(entry.value as number); break
    case "string": writer.byte(4); writer.string(entry.value as string); break
  }
}

type NormalizedRichSpan =
  | { type: "text"; text: string; attrs: AttrEntry[] }
  | { type: "embed"; value: ValueInput; attrs: AttrEntry[] }

function normalizedRichSpans(
  spans: readonly RichTextSpanInput[],
  operation: string,
  limits?: InputLimits,
): NormalizedRichSpan[] {
  if (!Array.isArray(spans)) throw new CollaError("invalid_value", operation, { reason: "RichText spans must be an array" })
  const values = ownArrayDataValues(spans, operation)
  const result: NormalizedRichSpan[] = []
  for (const span of values) {
    if (!isRecord(span)) throw new CollaError("invalid_value", operation, { reason: "RichText span must be a plain record" })
    const entries = ownDataEntries(span, operation)
    const type = entries.find(([key]) => key === "type")?.[1]
    const attrs = attrEntries(entries.find(([key]) => key === "attrs")?.[1] as AttrsData | undefined, operation, limits)
    if (type === "text") {
      if (entries.some(([key]) => !["type", "text", "attrs"].includes(key))) {
        throw new CollaError("invalid_value", operation, { reason: "unknown RichText Text span field" })
      }
      const value = entries.find(([key]) => key === "text")?.[1]
      if (typeof value !== "string") throw new CollaError("invalid_value", operation, { reason: "RichText Text span requires text" })
      assertWellFormedString(value, operation)
      const bytes = utf8.encode(value).length
      if (limits !== undefined && bytes > limits.maxStringBytes) {
        limitExceeded("string bytes", bytes, limits.maxStringBytes, operation)
      }
      if (value.length === 0) continue
      const previous = result.at(-1)
      if (previous?.type === "text" && sameAttrs(previous.attrs, attrs)) {
        previous.text += value
      } else {
        result.push({ type: "text", text: value, attrs })
      }
    } else if (type === "embed") {
      if (entries.some(([key]) => !["type", "value", "attrs"].includes(key))) {
        throw new CollaError("invalid_value", operation, { reason: "unknown RichText Embed span field" })
      }
      const valueEntry = entries.find(([key]) => key === "value")
      if (valueEntry === undefined) throw new CollaError("invalid_value", operation, { reason: "RichText Embed span requires value" })
      result.push({ type: "embed", value: valueEntry[1] as ValueInput, attrs })
    } else {
      throw new CollaError("invalid_value", operation, { reason: "unknown RichText span type" })
    }
  }
  if (limits !== undefined && result.length > limits.maxContainerLength) {
    limitExceeded("container length", result.length, limits.maxContainerLength, operation)
  }
  return result
}

export function richText(spans: readonly RichTextSpanInput[]): RichTextInput {
  const normalized = normalizedRichSpans(spans, "rich_text")
  const frozen = normalized.map(span => {
    const attrs = attrsData(span.attrs)
    return Object.freeze(span.type === "text"
      ? { type: "text" as const, text: span.text, ...(attrs === undefined ? {} : { attrs }) }
      : { type: "embed" as const, value: span.value, ...(attrs === undefined ? {} : { attrs }) })
  })
  return Object.freeze({ type: "richText", spans: Object.freeze(frozen) })
}

const inputLimitNames = Object.keys(DEFAULT_INPUT_LIMITS) as (keyof InputLimits)[]
const utf8 = new TextEncoder()

function isRecord(value: unknown): value is Record<string, unknown> {
  if (value === null || typeof value !== "object") return false
  const prototype = Object.getPrototypeOf(value)
  return prototype === Object.prototype || prototype === null
}

function ownDataEntries(
  value: Record<string, unknown>,
  operation: string,
): readonly (readonly [string, unknown])[] {
  const entries: [string, unknown][] = []
  for (const key of Reflect.ownKeys(value)) {
    if (typeof key !== "string") {
      throw new CollaError("invalid_value", operation, { reason: "symbol keys are not supported" })
    }
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    if (descriptor === undefined || !("value" in descriptor)) {
      throw new CollaError("invalid_value", operation, { reason: "accessor properties are not supported" })
    }
    entries.push([key, descriptor.value])
  }
  return entries
}

function ownArrayDataValues(value: readonly unknown[], operation: string): readonly unknown[] {
  const result: unknown[] = []
  for (const key of Reflect.ownKeys(value)) {
    if (typeof key !== "string") {
      throw new CollaError("invalid_value", operation, { reason: "symbol keys are not supported" })
    }
    if (key === "length") continue
    const index = Number(key)
    if (!Number.isSafeInteger(index) || index < 0 || index >= value.length || String(index) !== key) {
      throw new CollaError("invalid_value", operation, { reason: "array has non-index properties" })
    }
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    if (descriptor === undefined || !("value" in descriptor)) {
      throw new CollaError("invalid_value", operation, { reason: "array accessors are not supported" })
    }
    result[index] = descriptor.value
  }
  for (let index = 0; index < value.length; index += 1) {
    if (!Object.hasOwn(value, index)) {
      throw new CollaError("invalid_value", operation, { reason: "sparse arrays are not supported" })
    }
  }
  return result
}

function normalizeInputLimits(options: InputOptions | undefined, operation: string): InputLimits {
  if (options !== undefined && !isRecord(options)) {
    throw invalidArgument(operation, "options", "expected a plain record")
  }
  const optionEntries = options === undefined ? [] : ownDataEntries(options, operation)
  for (const [key] of optionEntries) {
    if (key !== "limits") throw invalidArgument(operation, "options", `unknown field ${key}`)
  }
  const overrides = options?.limits
  if (overrides !== undefined && !isRecord(overrides)) {
    throw invalidArgument(operation, "options.limits", "expected a plain record")
  }
  const result = { ...DEFAULT_INPUT_LIMITS }
  for (const [key, value] of overrides === undefined ? [] : ownDataEntries(overrides, operation)) {
    if (!inputLimitNames.includes(key as keyof InputLimits)) {
      throw invalidArgument(operation, "options.limits", `unknown field ${key}`)
    }
    if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
      throw invalidArgument(operation, `options.limits.${key}`, "expected a non-negative safe integer")
    }
    result[key as keyof InputLimits] = value
  }
  return Object.freeze(result)
}

function limitExceeded(limit: string, actual: number, maximum: number, operation: string): never {
  throw new CollaError("limit_exceeded", operation, { limit, actual, maximum })
}

class ByteWriter {
  readonly bytes: number[] = []

  byte(value: number): void {
    this.bytes.push(value)
  }

  varint(value: bigint): void {
    while (value >= 0x80n) {
      this.bytes.push(Number(value & 0x7fn) | 0x80)
      value >>= 7n
    }
    this.bytes.push(Number(value))
  }

  int64(value: bigint): void {
    this.varint(BigInt.asUintN(64, (value << 1n) ^ (value >> 63n)))
  }

  float64(value: number): void {
    const bytes = new Uint8Array(8)
    new DataView(bytes.buffer).setFloat64(0, Object.is(value, -0) ? 0 : value, true)
    this.bytes.push(...bytes)
  }

  string(value: string): void {
    const bytes = utf8.encode(value)
    this.varint(BigInt(bytes.length))
    this.bytes.push(...bytes)
  }

  blob(value: Uint8Array): void {
    this.varint(BigInt(value.length))
    this.bytes.push(...value)
  }
}

function compareUtf8(left: string, right: string): number {
  const a = utf8.encode(left)
  const b = utf8.encode(right)
  const length = Math.min(a.length, b.length)
  for (let index = 0; index < length; index += 1) {
    if (a[index] !== b[index]) return a[index] - b[index]
  }
  return a.length - b.length
}

function encodeValueInput(
  input: ValueInput,
  operation: string,
  limits?: InputLimits,
): Uint8Array {
  const writer = new ByteWriter()
  const active = new WeakSet<object>()
  let nodes = 0

  const check = (name: string, actual: number, maximum: number | undefined): void => {
    if (maximum !== undefined && actual > maximum) limitExceeded(name, actual, maximum, operation)
  }

  const encode = (value: ValueInput, depth: number): void => {
    nodes += 1
    check("value nodes", nodes, limits?.maxValueNodes)
    check("value depth", depth, limits?.maxDepth)

    if (value === null) {
      writer.byte(0)
    } else if (typeof value === "boolean") {
      writer.byte(value ? 2 : 1)
    } else if (typeof value === "bigint") {
      if (value < I64_MIN || value > I64_MAX) {
        throw new CollaError("invalid_value", operation, {
          reason: "integer is outside the signed 64-bit range",
        })
      }
      writer.byte(3)
      writer.int64(value)
    } else if (typeof value === "number") {
      if (!Number.isFinite(value)) {
        throw new CollaError("invalid_value", operation, { reason: "number must be finite" })
      }
      writer.byte(4)
      writer.float64(value)
    } else if (typeof value === "string") {
      assertWellFormedString(value, operation)
      check("string bytes", utf8.encode(value).length, limits?.maxStringBytes)
      writer.byte(5)
      writer.string(value)
    } else if (Array.isArray(value)) {
      if (active.has(value)) {
        throw new CollaError("invalid_value", operation, { reason: "cyclic ValueInput" })
      }
      check("container length", value.length, limits?.maxContainerLength)
      const values = ownArrayDataValues(value, operation)
      active.add(value)
      try {
        writer.byte(8)
        writer.varint(BigInt(value.length))
        for (const child of values) encode(child as ValueInput, depth + 1)
      } finally {
        active.delete(value)
      }
    } else if (isRecord(value)) {
      const entries = [...ownDataEntries(value, operation)]
      const marker = entries.find(([key]) => key === "type")
      if (marker?.[1] === "text") {
        if (entries.length !== 2 || !entries.some(([key]) => key === "value")) {
          throw new CollaError("invalid_value", operation, { reason: "invalid TextInput marker" })
        }
        const textValue = entries.find(([key]) => key === "value")?.[1]
        if (typeof textValue !== "string") {
          throw new CollaError("invalid_value", operation, { reason: "TextInput value must be a string" })
        }
        assertWellFormedString(textValue, operation)
        check("text bytes", utf8.encode(textValue).length, limits?.maxStringBytes)
        writer.byte(6)
        writer.string(textValue)
        return
      }
      if (marker?.[1] === "richText") {
        if (entries.length !== 2 || !entries.some(([key]) => key === "spans")) {
          throw new CollaError("invalid_value", operation, { reason: "invalid RichTextInput marker" })
        }
        if (active.has(value)) {
          throw new CollaError("invalid_value", operation, { reason: "cyclic ValueInput" })
        }
        const spans = normalizedRichSpans(
          entries.find(([key]) => key === "spans")?.[1] as readonly RichTextSpanInput[],
          operation,
          limits,
        )
        active.add(value)
        try {
          writer.byte(7)
          writer.varint(BigInt(spans.length))
          for (const span of spans) {
            if (span.type === "text") {
              writer.byte(0)
              writer.string(span.text)
            } else {
              writer.byte(1)
              encode(span.value, depth + 1)
            }
            writeAttrs(writer, span.attrs)
          }
        } finally {
          active.delete(value)
        }
        return
      }
      if (active.has(value)) {
        throw new CollaError("invalid_value", operation, { reason: "cyclic ValueInput" })
      }
      entries.sort(([left], [right]) => compareUtf8(left, right))
      check("container length", entries.length, limits?.maxContainerLength)
      for (const [key] of entries) {
        assertWellFormedString(key, operation)
        check("string bytes", utf8.encode(key).length, limits?.maxStringBytes)
      }
      active.add(value)
      try {
        writer.byte(9)
        writer.varint(BigInt(entries.length))
        for (const [key, child] of entries) {
          writer.string(key)
          encode(child as ValueInput, depth + 1)
        }
      } finally {
        active.delete(value)
      }
    } else {
      throw new CollaError("invalid_value", operation, { reason: "unsupported ValueInput" })
    }
  }

  encode(input, 1)
  return Uint8Array.from(writer.bytes)
}

function changeInputFields(
  value: unknown,
  allowed: readonly string[],
  required: readonly string[],
  operation: string,
  context: string,
): Map<string, unknown> {
  if (!isRecord(value)) throw invalidArgument(operation, context, "expected a plain record")
  const fields = new Map(ownDataEntries(value, operation))
  for (const key of fields.keys()) {
    if (!allowed.includes(key)) throw invalidArgument(operation, context, `unknown field ${key}`)
  }
  for (const key of required) {
    if (!fields.has(key)) throw invalidArgument(operation, context, `missing field ${key}`)
  }
  return fields
}

function changeInputArray(value: unknown, operation: string, context: string): readonly unknown[] {
  if (!Array.isArray(value)) throw invalidArgument(operation, context, "expected an array")
  return ownArrayDataValues(value, operation)
}

function changeInputLength(value: unknown, operation: string, context: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw invalidArgument(operation, context, "expected a non-negative safe integer")
  }
  return value
}

function writeChangeAttrPatch(
  writer: ByteWriter,
  input: unknown,
  operation: string,
): void {
  if (input === undefined) {
    writer.varint(0n)
    return
  }
  if (!isRecord(input)) throw invalidArgument(operation, "patch", "expected a plain record")
  const changes = ownDataEntries(input, operation).map(([key, value]) => {
    assertWellFormedString(key, operation)
    const fields = changeInputFields(value, ["type", "value"], ["type"], operation, `patch.${key}`)
    const type = fields.get("type")
    if (type === "remove") {
      if (fields.has("value")) throw invalidArgument(operation, `patch.${key}`, "remove must not include value")
      return { key, type: "remove" as const }
    }
    if (type !== "set" || !fields.has("value")) {
      throw invalidArgument(operation, `patch.${key}`, "expected set with value or remove")
    }
    const record = Object.create(null) as Record<string, AttrValueData>
    record[key] = fields.get("value") as AttrValueData
    const entry = attrEntries(record, operation)[0]
    return { key, type: "set" as const, entry }
  }).sort((left, right) => compareUtf8(left.key, right.key))

  writer.varint(BigInt(changes.length))
  for (const change of changes) {
    writer.string(change.key)
    if (change.type === "remove") {
      writer.byte(1)
    } else {
      writer.byte(0)
      writeAttrValue(writer, change.entry)
    }
  }
}

function encodeChangeInput(input: ChangeInput, operation: string): Uint8Array {
  const writer = new ByteWriter()
  const active = new WeakSet<object>()

  const encodeValue = (value: ValueInput): void => {
    writer.blob(encodeValueInput(value, operation))
  }

  const encode = (value: unknown, context: string): void => {
    const fields = changeInputFields(
      value,
      ["type", "value", "entries", "ops", "delta"],
      ["type"],
      operation,
      context,
    )
    const record = value as object
    if (active.has(record)) throw invalidArgument(operation, context, "cyclic ChangeInput")
    active.add(record)
    try {
      switch (fields.get("type")) {
        case "noop": {
          if (fields.size !== 1) throw invalidArgument(operation, context, "noop has unknown fields")
          writer.byte(0)
          break
        }
        case "replace": {
          if (fields.size !== 2 || !fields.has("value")) {
            throw invalidArgument(operation, context, "replace requires value")
          }
          writer.byte(1)
          encodeValue(fields.get("value") as ValueInput)
          break
        }
        case "map": {
          if (fields.size !== 2 || !fields.has("entries")) {
            throw invalidArgument(operation, context, "map requires entries")
          }
          const entries = changeInputArray(fields.get("entries"), operation, `${context}.entries`)
          writer.byte(2)
          writer.varint(BigInt(entries.length))
          entries.forEach((entry, index) => {
            const entryContext = `${context}.entries[${index}]`
            const item = changeInputFields(
              entry,
              ["key", "type", "value", "change"],
              ["key", "type"],
              operation,
              entryContext,
            )
            const key = item.get("key")
            if (typeof key !== "string") throw invalidArgument(operation, `${entryContext}.key`, "expected a string")
            assertWellFormedString(key, operation)
            writer.string(key)
            if (item.get("type") === "insert" && item.size === 3 && item.has("value")) {
              writer.byte(0)
              encodeValue(item.get("value") as ValueInput)
            } else if (item.get("type") === "delete" && item.size === 2) {
              writer.byte(1)
            } else if (item.get("type") === "modify" && item.size === 3 && item.has("change")) {
              writer.byte(2)
              encode(item.get("change"), `${entryContext}.change`)
            } else {
              throw invalidArgument(operation, entryContext, "invalid map entry")
            }
          })
          break
        }
        case "list": {
          if (fields.size !== 2 || !fields.has("ops")) {
            throw invalidArgument(operation, context, "list requires ops")
          }
          const ops = changeInputArray(fields.get("ops"), operation, `${context}.ops`)
          writer.byte(3)
          writer.varint(BigInt(ops.length))
          ops.forEach((op, index) => {
            const opContext = `${context}.ops[${index}]`
            const item = changeInputFields(
              op,
              ["type", "length", "values", "change"],
              ["type"],
              operation,
              opContext,
            )
            if (item.get("type") === "retain" && item.size === 2 && item.has("length")) {
              writer.byte(0)
              writer.varint(BigInt(changeInputLength(item.get("length"), operation, `${opContext}.length`)))
            } else if (item.get("type") === "insert" && item.size === 2 && item.has("values")) {
              const values = changeInputArray(item.get("values"), operation, `${opContext}.values`)
              writer.byte(1)
              writer.varint(BigInt(values.length))
              values.forEach(value => encodeValue(value as ValueInput))
            } else if (item.get("type") === "delete" && item.size === 2 && item.has("length")) {
              writer.byte(2)
              writer.varint(BigInt(changeInputLength(item.get("length"), operation, `${opContext}.length`)))
            } else if (item.get("type") === "modify" && item.size === 2 && item.has("change")) {
              writer.byte(3)
              encode(item.get("change"), `${opContext}.change`)
            } else {
              throw invalidArgument(operation, opContext, "invalid list operation")
            }
          })
          break
        }
        case "text": {
          if (fields.size !== 2 || !fields.has("ops")) {
            throw invalidArgument(operation, context, "text requires ops")
          }
          const ops = changeInputArray(fields.get("ops"), operation, `${context}.ops`)
          writer.byte(4)
          writer.varint(BigInt(ops.length))
          ops.forEach((op, index) => {
            const opContext = `${context}.ops[${index}]`
            const item = changeInputFields(op, ["type", "length", "text"], ["type"], operation, opContext)
            if (item.get("type") === "retain" && item.size === 2 && item.has("length")) {
              writer.byte(0)
              writer.varint(BigInt(changeInputLength(item.get("length"), operation, `${opContext}.length`)))
            } else if (item.get("type") === "insert" && item.size === 2 && item.has("text")) {
              const text = item.get("text")
              if (typeof text !== "string") throw invalidArgument(operation, `${opContext}.text`, "expected a string")
              assertWellFormedString(text, operation)
              writer.byte(1)
              writer.string(text)
            } else if (item.get("type") === "delete" && item.size === 2 && item.has("length")) {
              writer.byte(2)
              writer.varint(BigInt(changeInputLength(item.get("length"), operation, `${opContext}.length`)))
            } else {
              throw invalidArgument(operation, opContext, "invalid text operation")
            }
          })
          break
        }
        case "richText": {
          if (fields.size !== 2 || !fields.has("ops")) {
            throw invalidArgument(operation, context, "richText requires ops")
          }
          const ops = changeInputArray(fields.get("ops"), operation, `${context}.ops`)
          writer.byte(5)
          writer.varint(BigInt(ops.length))
          ops.forEach((op, index) => {
            const opContext = `${context}.ops[${index}]`
            const item = changeInputFields(
              op,
              ["type", "length", "patch", "content"],
              ["type"],
              operation,
              opContext,
            )
            if (
              item.get("type") === "retain" &&
              item.has("length") &&
              !item.has("content") &&
              item.size <= 3
            ) {
              writer.byte(0)
              writer.varint(BigInt(changeInputLength(item.get("length"), operation, `${opContext}.length`)))
              writeChangeAttrPatch(writer, item.get("patch"), operation)
            } else if (item.get("type") === "insert" && item.size === 2 && item.has("content")) {
              const contentContext = `${opContext}.content`
              const content = changeInputFields(
                item.get("content"),
                ["type", "text", "value", "attrs"],
                ["type"],
                operation,
                contentContext,
              )
              writer.byte(1)
              if (
                content.get("type") === "text" &&
                content.has("text") &&
                !content.has("value") &&
                content.size <= 3
              ) {
                const text = content.get("text")
                if (typeof text !== "string") throw invalidArgument(operation, `${contentContext}.text`, "expected a string")
                assertWellFormedString(text, operation)
                writer.byte(0)
                writer.string(text)
              } else if (
                content.get("type") === "embed" &&
                content.has("value") &&
                !content.has("text") &&
                content.size <= 3
              ) {
                writer.byte(1)
                encodeValue(content.get("value") as ValueInput)
              } else {
                throw invalidArgument(operation, contentContext, "invalid RichText content")
              }
              writeAttrs(writer, attrEntries(content.get("attrs") as AttrsData | undefined, operation))
            } else if (item.get("type") === "delete" && item.size === 2 && item.has("length")) {
              writer.byte(2)
              writer.varint(BigInt(changeInputLength(item.get("length"), operation, `${opContext}.length`)))
            } else {
              throw invalidArgument(operation, opContext, "invalid richText operation")
            }
          })
          break
        }
        case "int": {
          if (fields.size !== 2 || !fields.has("delta")) {
            throw invalidArgument(operation, context, "int requires delta")
          }
          const delta = fields.get("delta")
          if (typeof delta !== "bigint" || delta < I64_MIN || delta > I64_MAX) {
            throw invalidArgument(operation, `${context}.delta`, "expected a signed 64-bit bigint")
          }
          writer.byte(6)
          writer.int64(delta)
          break
        }
        default:
          throw invalidArgument(operation, `${context}.type`, "unknown ChangeInput type")
      }
    } finally {
      active.delete(record)
    }
  }

  encode(input, "change")
  return Uint8Array.from(writer.bytes)
}

class ByteReader {
  #offset = 0

  constructor(private readonly bytes: Uint8Array, private readonly operation: string) {}

  byte(): number {
    const value = this.bytes[this.#offset]
    if (value === undefined) this.fail("unexpected end of encoded Value")
    this.#offset += 1
    return value
  }

  exact(length: number): Uint8Array {
    const end = this.#offset + length
    if (!Number.isSafeInteger(end) || end > this.bytes.length) this.fail("unexpected end of encoded Value")
    const value = this.bytes.slice(this.#offset, end)
    this.#offset = end
    return value
  }

  varint(): bigint {
    let value = 0n
    for (let index = 0; index < 10; index += 1) {
      const byte = this.byte()
      value |= BigInt(byte & 0x7f) << BigInt(index * 7)
      if ((byte & 0x80) === 0) return value
    }
    return this.fail("integer is out of range")
  }

  length(): number {
    const value = this.varint()
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) this.fail("length is out of range")
    return Number(value)
  }

  string(): string {
    try {
      return new TextDecoder("utf-8", { fatal: true }).decode(this.exact(this.length()))
    } catch (error) {
      if (error instanceof CollaError) throw error
      return this.fail("invalid UTF-8")
    }
  }

  finish(): void {
    if (this.#offset !== this.bytes.length) this.fail("trailing encoded Value bytes")
  }

  fail(reason: string): never {
    throw new CollaError("invalid_encoding", this.operation, { reason, offset: this.#offset })
  }
}

function readAttrs(reader: ByteReader): AttrsData | undefined {
  const result = Object.create(null) as Record<string, AttrValueData>
  const length = reader.length()
  for (let index = 0; index < length; index += 1) {
    const key = reader.string()
    switch (reader.byte()) {
      case 0: result[key] = false; break
      case 1: result[key] = true; break
      case 2: {
        const value = reader.varint()
        result[key] = (value >> 1n) ^ -(value & 1n)
        break
      }
      case 3: {
        const bytes = reader.exact(8)
        result[key] = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getFloat64(0, true)
        break
      }
      case 4: result[key] = reader.string(); break
      default: return reader.fail("unsupported attribute value kind")
    }
  }
  return length === 0 ? undefined : Object.freeze(result)
}

function decodeValueData(bytes: Uint8Array, operation: string): ValueData {
  const reader = new ByteReader(bytes, operation)
  const decode = (): ValueData => {
    switch (reader.byte()) {
      case 0: return null
      case 1: return false
      case 2: return true
      case 3: {
        const value = reader.varint()
        return (value >> 1n) ^ -(value & 1n)
      }
      case 4: {
        const bytes = reader.exact(8)
        return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getFloat64(0, true)
      }
      case 5: return reader.string()
      case 6: return Object.freeze({ type: "text" as const, value: reader.string() })
      case 7: {
        const spans: RichTextSpanData[] = []
        const length = reader.length()
        for (let index = 0; index < length; index += 1) {
          const kind = reader.byte()
          if (kind === 0) {
            const text = reader.string()
            const attrs = readAttrs(reader)
            spans.push(Object.freeze({
              type: "text",
              text,
              ...(attrs === undefined ? {} : { attrs }),
            }))
          } else if (kind === 1) {
            const value = decode()
            const attrs = readAttrs(reader)
            spans.push(Object.freeze({
              type: "embed",
              value,
              ...(attrs === undefined ? {} : { attrs }),
            }))
          } else {
            reader.fail("unsupported RichText content kind")
          }
        }
        return Object.freeze({ type: "richText", spans: Object.freeze(spans) })
      }
      case 8: {
        const values: ValueData[] = []
        const length = reader.length()
        for (let index = 0; index < length; index += 1) values.push(decode())
        return Object.freeze(values)
      }
      case 9: {
        const result = Object.create(null) as Record<string, ValueData>
        const length = reader.length()
        for (let index = 0; index < length; index += 1) result[reader.string()] = decode()
        return Object.freeze(result)
      }
      default: return reader.fail("unsupported encoded Value kind")
    }
  }
  const value = decode()
  reader.finish()
  return value
}

function pathJson(path: Path, operation: string): string {
  if (!Array.isArray(path)) throw invalidArgument(operation, "path", "expected an array")
  const segments = path.map((segment, index) => {
    if (typeof segment === "string") return segment
    if (typeof segment === "number" && Number.isSafeInteger(segment) && segment >= 0) return segment
    throw invalidArgument(operation, `path[${index}]`, "expected a string or non-negative safe integer")
  })
  return JSON.stringify(segments)
}

export class Value {
  #handle: ValueHandle | undefined

  private constructor(handle: ValueHandle) {
    this.#handle = handle
    valueFinalizer.register(this, handle, this)
  }

  static fromJS(input: ValueInput, options?: InputOptions): Value {
    try {
      const limits = normalizeInputLimits(options, "value_from_js")
      const bytes = encodeValueInput(input, "value_from_js", limits)
      return new Value(ValueHandle.decode(bytes, JSON.stringify(limits)))
    } catch (error) {
      throw fromWasmError(error, "value_from_js")
    }
  }

  static decode(bytes: Uint8Array, options?: InputOptions): Value {
    if (!(bytes instanceof Uint8Array)) {
      throw invalidArgument("value_decode", "bytes", "expected Uint8Array")
    }
    try {
      const limits = normalizeInputLimits(options, "value_decode")
      return new Value(ValueHandle.decode(bytes, JSON.stringify(limits)))
    } catch (error) {
      throw fromWasmError(error, "value_decode")
    }
  }

  kind(path: Path = []): ValueKind {
    try {
      return this.#get("value_kind").kind(pathJson(path, "value_kind")) as ValueKind
    } catch (error) {
      throw fromWasmError(error, "value_kind", path)
    }
  }

  has(path: Path): boolean {
    try {
      return this.#get("value_has").has(pathJson(path, "value_has"))
    } catch (error) {
      throw fromWasmError(error, "value_has", path)
    }
  }

  get(path: Path): ValueData {
    try {
      const bytes = this.#get("value_get").getBytes(pathJson(path, "value_get"))
      return decodeValueData(new Uint8Array(bytes), "value_get")
    } catch (error) {
      throw fromWasmError(error, "value_get", path)
    }
  }

  toJS(): ValueData {
    return decodeValueData(this.encode(), "value_to_js")
  }

  encode(): Uint8Array {
    return new Uint8Array(this.#get("value_encode").encode())
  }

  clone(): Value {
    return new Value(this.#get("value_clone").cloneHandle())
  }

  dispose(): void {
    const handle = this.#handle
    if (handle === undefined) return
    this.#handle = undefined
    valueFinalizer.unregister(this)
    handle.free()
  }

  [Symbol.dispose](): void {
    this.dispose()
  }

  #get(operation: string): ValueHandle {
    if (this.#handle === undefined) {
      throw invalidState(operation, "Value", "disposed")
    }
    return this.#handle
  }

  /** @internal */
  static _handle(value: Value, operation: string): ValueHandle {
    if (!(value instanceof Value)) {
      throw invalidArgument(operation, "value", "expected Value")
    }
    return value.#get(operation)
  }

  /** @internal */
  static _fromHandle(handle: ValueHandle): Value {
    return new Value(handle)
  }

}

export class Change {
  #handle: ChangeHandle | undefined

  private constructor(handle: ChangeHandle) {
    this.#handle = handle
    changeFinalizer.register(this, handle, this)
  }

  static fromJS(input: ChangeInput, options?: InputOptions): Change {
    try {
      const limits = normalizeInputLimits(options, "change_from_js")
      const bytes = encodeChangeInput(input, "change_from_js")
      return new Change(ChangeHandle.fromInput(bytes, JSON.stringify(limits)))
    } catch (error) {
      throw fromWasmError(error, "change_from_js")
    }
  }

  static build(
    edit: (change: ChangeBuilder) => unknown,
    options?: InputOptions,
  ): Change {
    if (typeof edit !== "function") {
      throw invalidArgument("change_build", "edit", "expected a function")
    }
    return Change.fromJS(buildChangeInput(edit), options)
  }

  static decode(bytes: Uint8Array, options?: InputOptions): Change {
    if (!(bytes instanceof Uint8Array)) {
      throw invalidArgument("change_decode", "bytes", "expected Uint8Array")
    }
    try {
      const limits = normalizeInputLimits(options, "change_decode")
      return new Change(ChangeHandle.decode(bytes, JSON.stringify(limits)))
    } catch (error) {
      throw fromWasmError(error, "change_decode")
    }
  }

  encode(): Uint8Array {
    return new Uint8Array(this.#get("change_encode").encode())
  }

  clone(): Change {
    return new Change(this.#get("change_clone").cloneHandle())
  }

  dispose(): void {
    const handle = this.#handle
    if (handle === undefined) return
    this.#handle = undefined
    changeFinalizer.unregister(this)
    handle.free()
  }

  [Symbol.dispose](): void {
    this.dispose()
  }

  #get(operation: string): ChangeHandle {
    if (this.#handle === undefined) {
      throw invalidState(operation, "Change", "disposed")
    }
    return this.#handle
  }

  /** @internal */
  static _fromHandle(handle: ChangeHandle): Change {
    return new Change(handle)
  }

  /** @internal */
  static _handle(change: Change, operation: string): ChangeHandle {
    if (!(change instanceof Change)) {
      throw invalidArgument(operation, "change", "expected Change")
    }
    return change.#get(operation)
  }
}

export interface Range {
  readonly from: number
  readonly to: number
}

interface ChangeViewEntryBase {
  readonly path: Path
}

export type AttrPatchView = Readonly<Record<string,
  | { readonly type: "set"; readonly value: AttrValueData }
  | { readonly type: "remove" }
>>

export type ChangeViewEntry =
  | (ChangeViewEntryBase & { readonly type: "value.replace"; readonly value: ValueData })
  | (ChangeViewEntryBase & { readonly type: "int.add"; readonly delta: bigint })
  | (ChangeViewEntryBase & { readonly type: "map.set"; readonly key: string; readonly value: ValueData })
  | (ChangeViewEntryBase & { readonly type: "map.delete"; readonly key: string })
  | (ChangeViewEntryBase & { readonly type: "list.insert"; readonly index: number; readonly values: readonly ValueData[] })
  | (ChangeViewEntryBase & { readonly type: "list.set"; readonly index: number; readonly value: ValueData })
  | (ChangeViewEntryBase & { readonly type: "list.delete"; readonly range: Range })
  | (ChangeViewEntryBase & { readonly type: "text.insert"; readonly at: number; readonly text: string })
  | (ChangeViewEntryBase & { readonly type: "text.delete"; readonly range: Range })
  | (ChangeViewEntryBase & { readonly type: "richText.insertText"; readonly at: number; readonly text: string; readonly attrs?: AttrsData })
  | (ChangeViewEntryBase & { readonly type: "richText.insertEmbed"; readonly at: number; readonly embed: ValueData; readonly attrs?: AttrsData })
  | (ChangeViewEntryBase & { readonly type: "richText.delete"; readonly range: Range })
  | (ChangeViewEntryBase & { readonly type: "richText.format"; readonly range: Range; readonly patch: AttrPatchView })

export type ChangeView = readonly ChangeViewEntry[]

export interface ChangeBuilder {
  noop(): this
  replace(value: ValueInput): this
  map(edit: (map: MapChangeBuilder) => unknown): this
  list(edit: (list: ListChangeBuilder) => unknown): this
  text(edit: (text: TextChangeBuilder) => unknown): this
  richText(edit: (richText: RichTextChangeBuilder) => unknown): this
  intAdd(delta: bigint): this
}

export interface MapChangeBuilder {
  insert(key: string, value: ValueInput): this
  delete(key: string): this
  modify(key: string, edit: (change: ChangeBuilder) => unknown): this
}

export interface ListChangeBuilder {
  retain(length: number): this
  insert(values: readonly ValueInput[]): this
  delete(length: number): this
  modify(edit: (change: ChangeBuilder) => unknown): this
}

export interface TextChangeBuilder {
  retain(length: number): this
  insert(text: string): this
  delete(length: number): this
}

export interface RichTextChangeBuilder {
  retain(length: number, edit?: (patch: AttrPatchBuilder) => unknown): this
  insertText(text: string, attrs?: AttrsData): this
  insertEmbed(value: ValueInput, attrs?: AttrsData): this
  delete(length: number): this
}

export interface AttrPatchBuilder {
  set(key: string, value: AttrValueData): this
  remove(key: string): this
}

abstract class ChangeBuildScope {
  #active = true

  close(): void {
    this.#active = false
  }

  protected assertActive(): void {
    if (!this.#active) throw invalidState("change_build", "ChangeBuilder", "scope_closed")
  }
}

function runBuildCallback<T extends ChangeBuildScope, R>(
  scope: T,
  edit: (scope: T) => unknown,
  finish: () => R,
): R {
  if (typeof edit !== "function") {
    scope.close()
    throw invalidArgument("change_build", "edit", "expected a function")
  }
  try {
    const result = edit(scope)
    if (
      result !== null &&
      (typeof result === "object" || typeof result === "function") &&
      typeof (result as { then?: unknown }).then === "function"
    ) {
      throw invalidArgument("change_build", "edit", "callback must be synchronous")
    }
    return finish()
  } finally {
    scope.close()
  }
}

class RootChangeScope extends ChangeBuildScope implements ChangeBuilder {
  #input: ChangeInput | undefined

  noop(): this {
    return this.#select(Object.freeze({ type: "noop" }))
  }

  replace(value: ValueInput): this {
    return this.#select(Object.freeze({ type: "replace", value }))
  }

  map(edit: (map: MapChangeBuilder) => unknown): this {
    this.#assertUnselected()
    const scope = new MapChangeScope()
    const entries = runBuildCallback(scope, edit, () => scope.finish())
    return this.#select(Object.freeze({ type: "map", entries }))
  }

  list(edit: (list: ListChangeBuilder) => unknown): this {
    this.#assertUnselected()
    const scope = new ListChangeScope()
    const ops = runBuildCallback(scope, edit, () => scope.finish())
    return this.#select(Object.freeze({ type: "list", ops }))
  }

  text(edit: (text: TextChangeBuilder) => unknown): this {
    this.#assertUnselected()
    const scope = new TextChangeScope()
    const ops = runBuildCallback(scope, edit, () => scope.finish())
    return this.#select(Object.freeze({ type: "text", ops }))
  }

  richText(edit: (richText: RichTextChangeBuilder) => unknown): this {
    this.#assertUnselected()
    const scope = new RichTextChangeScope()
    const ops = runBuildCallback(scope, edit, () => scope.finish())
    return this.#select(Object.freeze({ type: "richText", ops }))
  }

  intAdd(delta: bigint): this {
    return this.#select(Object.freeze({ type: "int", delta }))
  }

  finish(): ChangeInput {
    this.assertActive()
    if (this.#input === undefined) {
      throw invalidArgument("change_build", "edit", "missing_change_kind")
    }
    return this.#input
  }

  #select(input: ChangeInput): this {
    this.#assertUnselected()
    this.#input = input
    return this
  }

  #assertUnselected(): void {
    this.assertActive()
    if (this.#input !== undefined) {
      throw invalidArgument("change_build", "edit", "duplicate_change_kind")
    }
  }
}

class MapChangeScope extends ChangeBuildScope implements MapChangeBuilder {
  readonly #entries: MapChangeEntryInput[] = []

  insert(key: string, value: ValueInput): this {
    this.assertActive()
    this.#entries.push(Object.freeze({ key, type: "insert", value }))
    return this
  }

  delete(key: string): this {
    this.assertActive()
    this.#entries.push(Object.freeze({ key, type: "delete" }))
    return this
  }

  modify(key: string, edit: (change: ChangeBuilder) => unknown): this {
    this.assertActive()
    const child = new RootChangeScope()
    const change = runBuildCallback(child, edit, () => child.finish())
    this.#entries.push(Object.freeze({ key, type: "modify", change }))
    return this
  }

  finish(): readonly MapChangeEntryInput[] {
    this.assertActive()
    return Object.freeze([...this.#entries])
  }
}

class ListChangeScope extends ChangeBuildScope implements ListChangeBuilder {
  readonly #ops: ListChangeOpInput[] = []

  retain(length: number): this {
    this.assertActive()
    this.#ops.push(Object.freeze({ type: "retain", length }))
    return this
  }

  insert(values: readonly ValueInput[]): this {
    this.assertActive()
    if (!Array.isArray(values)) {
      throw invalidArgument("change_build", "values", "expected an array")
    }
    this.#ops.push(Object.freeze({
      type: "insert",
      values: Object.freeze([...ownArrayDataValues(values, "change_build")]) as readonly ValueInput[],
    }))
    return this
  }

  delete(length: number): this {
    this.assertActive()
    this.#ops.push(Object.freeze({ type: "delete", length }))
    return this
  }

  modify(edit: (change: ChangeBuilder) => unknown): this {
    this.assertActive()
    const child = new RootChangeScope()
    const change = runBuildCallback(child, edit, () => child.finish())
    this.#ops.push(Object.freeze({ type: "modify", change }))
    return this
  }

  finish(): readonly ListChangeOpInput[] {
    this.assertActive()
    return Object.freeze([...this.#ops])
  }
}

class TextChangeScope extends ChangeBuildScope implements TextChangeBuilder {
  readonly #ops: TextChangeOpInput[] = []

  retain(length: number): this {
    this.assertActive()
    this.#ops.push(Object.freeze({ type: "retain", length }))
    return this
  }

  insert(text: string): this {
    this.assertActive()
    this.#ops.push(Object.freeze({ type: "insert", text }))
    return this
  }

  delete(length: number): this {
    this.assertActive()
    this.#ops.push(Object.freeze({ type: "delete", length }))
    return this
  }

  finish(): readonly TextChangeOpInput[] {
    this.assertActive()
    return Object.freeze([...this.#ops])
  }
}

class PatchChangeScope extends ChangeBuildScope implements AttrPatchBuilder {
  readonly #patch = Object.create(null) as Record<
    string,
    { readonly type: "set"; readonly value: AttrValueData } | { readonly type: "remove" }
  >

  set(key: string, value: AttrValueData): this {
    this.assertActive()
    this.#patch[key] = Object.freeze({ type: "set", value })
    return this
  }

  remove(key: string): this {
    this.assertActive()
    this.#patch[key] = Object.freeze({ type: "remove" })
    return this
  }

  finish(): AttrPatchInput {
    this.assertActive()
    return Object.freeze(this.#patch)
  }
}

class RichTextChangeScope extends ChangeBuildScope implements RichTextChangeBuilder {
  readonly #ops: RichTextChangeOpInput[] = []

  retain(length: number, edit?: (patch: AttrPatchBuilder) => unknown): this {
    this.assertActive()
    if (edit === undefined) {
      this.#ops.push(Object.freeze({ type: "retain", length }))
    } else {
      const scope = new PatchChangeScope()
      const patch = runBuildCallback(scope, edit, () => scope.finish())
      this.#ops.push(Object.freeze({ type: "retain", length, patch }))
    }
    return this
  }

  insertText(text: string, attrs?: AttrsData): this {
    this.assertActive()
    const content = Object.freeze({
      type: "text" as const,
      text,
      ...(attrs === undefined ? {} : { attrs }),
    })
    this.#ops.push(Object.freeze({ type: "insert", content }))
    return this
  }

  insertEmbed(value: ValueInput, attrs?: AttrsData): this {
    this.assertActive()
    const content = Object.freeze({
      type: "embed" as const,
      value,
      ...(attrs === undefined ? {} : { attrs }),
    })
    this.#ops.push(Object.freeze({ type: "insert", content }))
    return this
  }

  delete(length: number): this {
    this.assertActive()
    this.#ops.push(Object.freeze({ type: "delete", length }))
    return this
  }

  finish(): readonly RichTextChangeOpInput[] {
    this.assertActive()
    return Object.freeze([...this.#ops])
  }
}

function buildChangeInput(edit: (change: ChangeBuilder) => unknown): ChangeInput {
  const scope = new RootChangeScope()
  return runBuildCallback(scope, edit, () => scope.finish())
}

function indexArgument(value: unknown, operation: string, argument: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw invalidArgument(operation, argument, "expected a non-negative safe integer")
  }
  return value
}

export function apply(base: Value, change: Change): Value {
  try {
    return Value._fromHandle(
      applyHandles(
        Value._handle(base, "apply"),
        Change._handle(change, "apply"),
      ),
    )
  } catch (error) {
    throw fromWasmError(error, "apply")
  }
}

export function compose(first: Change, second: Change): Change {
  try {
    return Change._fromHandle(composeHandles(
      Change._handle(first, "compose"),
      Change._handle(second, "compose"),
    ))
  } catch (error) {
    throw fromWasmError(error, "compose")
  }
}

export function invert(change: Change, base: Value): Change {
  try {
    return Change._fromHandle(invertHandle(
      Change._handle(change, "invert"),
      Value._handle(base, "invert"),
    ))
  } catch (error) {
    throw fromWasmError(error, "invert")
  }
}

export interface TransformPairOptions {
  readonly order: "left-first" | "right-first"
}

export function transformPair(
  left: Change,
  right: Change,
  options: TransformPairOptions,
): readonly [Change, Change] {
  const operation = "transform_pair"
  if (!isRecord(options)) throw invalidArgument(operation, "options", "expected a plain record")
  const entries = ownDataEntries(options, operation)
  if (entries.length !== 1 || entries[0][0] !== "order") {
    throw invalidArgument(operation, "options", "expected only the order field")
  }
  const order = entries[0][1]
  if (order !== "left-first" && order !== "right-first") {
    throw invalidArgument(operation, "options.order", "expected left-first or right-first")
  }
  try {
    const pair = transformPairHandles(
      Change._handle(left, operation),
      Change._handle(right, operation),
      order === "left-first",
    )
    try {
      return Object.freeze([
        Change._fromHandle(pair.leftHandle()),
        Change._fromHandle(pair.rightHandle()),
      ])
    } finally {
      pair.free()
    }
  } catch (error) {
    throw fromWasmError(error, operation)
  }
}

type RawChangeViewEntry = {
  type: ChangeViewEntry["type"]
  path: (string | number)[]
  key?: string
  index?: number
  at?: number
  from?: number
  to?: number
  text?: string
  delta?: string
  valueBytes?: number[]
  embedBytes?: number[]
  valuesBytes?: number[][]
  attrs?: AttrEntry[]
  patch?: ({ key: string; action: "remove" } | ({ action: "set" } & AttrEntry))[]
}

function viewValue(bytes: number[] | undefined): ValueData {
  return decodeValueData(Uint8Array.from(bytes ?? []), "inspect_change")
}

function viewRange(entry: RawChangeViewEntry): Range {
  return Object.freeze({ from: entry.from ?? 0, to: entry.to ?? 0 })
}

function viewAttrs(entries: AttrEntry[] | undefined): AttrsData | undefined {
  return attrsData(entries ?? [])
}

function viewPatch(entries: RawChangeViewEntry["patch"]): AttrPatchView {
  const patch = Object.create(null) as Record<string,
    | { readonly type: "set"; readonly value: AttrValueData }
    | { readonly type: "remove" }
  >
  for (const entry of entries ?? []) {
    patch[entry.key] = entry.action === "remove"
      ? Object.freeze({ type: "remove" as const })
      : Object.freeze({
          type: "set" as const,
          value: entry.kind === "int" ? BigInt(entry.value) : entry.value,
        })
  }
  return Object.freeze(patch)
}

export function inspectChange(change: Change, base: Value): ChangeView {
  const operation = "inspect_change"
  try {
    const raw = JSON.parse(inspectChangeHandle(
      Change._handle(change, operation),
      Value._handle(base, operation),
    )) as RawChangeViewEntry[]
    const view = raw.map((entry): ChangeViewEntry => {
      const path = Object.freeze([...entry.path])
      switch (entry.type) {
        case "value.replace": return Object.freeze({ type: entry.type, path, value: viewValue(entry.valueBytes) })
        case "int.add": return Object.freeze({ type: entry.type, path, delta: BigInt(entry.delta ?? "0") })
        case "map.set": return Object.freeze({ type: entry.type, path, key: entry.key ?? "", value: viewValue(entry.valueBytes) })
        case "map.delete": return Object.freeze({ type: entry.type, path, key: entry.key ?? "" })
        case "list.insert": return Object.freeze({
          type: entry.type,
          path,
          index: entry.index ?? 0,
          values: Object.freeze((entry.valuesBytes ?? []).map(viewValue)),
        })
        case "list.set": return Object.freeze({ type: entry.type, path, index: entry.index ?? 0, value: viewValue(entry.valueBytes) })
        case "list.delete": return Object.freeze({ type: entry.type, path, range: viewRange(entry) })
        case "text.insert": return Object.freeze({ type: entry.type, path, at: entry.at ?? 0, text: entry.text ?? "" })
        case "text.delete": return Object.freeze({ type: entry.type, path, range: viewRange(entry) })
        case "richText.insertText": {
          const attrs = viewAttrs(entry.attrs)
          return Object.freeze({
            type: entry.type,
            path,
            at: entry.at ?? 0,
            text: entry.text ?? "",
            ...(attrs === undefined ? {} : { attrs }),
          })
        }
        case "richText.insertEmbed": {
          const attrs = viewAttrs(entry.attrs)
          return Object.freeze({
            type: entry.type,
            path,
            at: entry.at ?? 0,
            embed: viewValue(entry.embedBytes),
            ...(attrs === undefined ? {} : { attrs }),
          })
        }
        case "richText.delete": return Object.freeze({ type: entry.type, path, range: viewRange(entry) })
        case "richText.format": return Object.freeze({
          type: entry.type,
          path,
          range: viewRange(entry),
          patch: viewPatch(entry.patch),
        })
      }
    })
    return Object.freeze(view)
  } catch (error) {
    throw fromWasmError(error, operation)
  }
}

export function resolveCodePointPosition(
  value: Value,
  path: Path,
  utf16Position: number,
): number {
  const operation = "resolve_code_point_position"
  const position = indexArgument(utf16Position, operation, "utf16Position")
  try {
    return resolveCodePointPositionHandle(
      Value._handle(value, operation),
      pathJson(path, operation),
      position,
    )
  } catch (error) {
    throw fromWasmError(error, operation, path)
  }
}

export function resolveUtf16Position(
  value: Value,
  path: Path,
  codePointPosition: number,
): number {
  const operation = "resolve_utf16_position"
  const position = indexArgument(codePointPosition, operation, "codePointPosition")
  try {
    return resolveUtf16PositionHandle(
      Value._handle(value, operation),
      pathJson(path, operation),
      position,
    )
  } catch (error) {
    throw fromWasmError(error, operation, path)
  }
}
