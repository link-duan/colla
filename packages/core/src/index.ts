import {
  applyHandles,
  ChangeHandle,
  composeHandles,
  convertChangeToEditStepsHandle,
  inspectChangeHandle,
  invertHandle,
  resolveCodePointPositionHandle,
  resolveUtf16PositionHandle,
  transformPairHandles,
  ValueHandle as WasmValueHandle,
} from "./internal/colla_wasm.js"

export type Path = readonly (string | number)[]
export type ValueKind =
  | "null"
  | "bool"
  | "int"
  | "float"
  | "string"
  | "text"
  | "richtext"
  | "list"
  | "map"

export type Value =
  | null
  | boolean
  | bigint
  | number
  | string
  | Text
  | RichText
  | readonly Value[]
  | ValueMap
export interface ValueMap {
  readonly [key: string]: Value
}
export interface Text {
  readonly type: "text"
  readonly value: string
}
export type AttrValueData = boolean | bigint | number | string
export interface AttrsData {
  readonly [key: string]: AttrValueData
}
export type RichTextSpan =
  | { readonly type: "text"; readonly text: string; readonly attrs?: AttrsData }
  | { readonly type: "embed"; readonly value: Value; readonly attrs?: AttrsData }
export interface RichText {
  readonly type: "richtext"
  readonly spans: readonly RichTextSpan[]
}

export type ChangeInput =
  | { readonly type: "noop" }
  | { readonly type: "replace"; readonly value: Value }
  | { readonly type: "map"; readonly entries: readonly MapChangeEntryInput[] }
  | { readonly type: "list"; readonly ops: readonly ListChangeOpInput[] }
  | { readonly type: "text"; readonly ops: readonly TextChangeOpInput[] }
  | { readonly type: "richtext"; readonly ops: readonly RichTextChangeOpInput[] }
  | { readonly type: "int"; readonly delta: bigint }

export type MapChangeEntryInput =
  | { readonly key: string; readonly type: "insert"; readonly value: Value }
  | { readonly key: string; readonly type: "delete" }
  | { readonly key: string; readonly type: "modify"; readonly change: ChangeInput }

export type ListChangeOpInput =
  | { readonly type: "retain"; readonly length: number }
  | { readonly type: "insert"; readonly values: readonly Value[] }
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
  | { readonly type: "insert"; readonly content: RichTextSpan }
  | { readonly type: "delete"; readonly length: number }

export type MapEditOp =
  | { readonly type: "insert"; readonly value: Value }
  | { readonly type: "delete" }

export type ListEditOp =
  | { readonly type: "retain"; readonly length: number }
  | { readonly type: "insert"; readonly values: readonly Value[] }
  | { readonly type: "delete"; readonly length: number }
  | { readonly type: "modify"; readonly steps: readonly EditStep[] }

export type TextEditOp =
  | { readonly type: "retain"; readonly length: number }
  | { readonly type: "insert"; readonly text: string }
  | { readonly type: "delete"; readonly length: number }

export type RichTextEditOp =
  | { readonly type: "retain"; readonly length: number; readonly patch?: AttrPatchView }
  | { readonly type: "insert"; readonly span: RichTextSpan }
  | { readonly type: "delete"; readonly length: number }

export type EditStep =
  | { readonly type: "replace"; readonly path: Path; readonly value: Value }
  | { readonly type: "int"; readonly path: Path; readonly delta: bigint }
  | { readonly type: "map"; readonly path: Path; readonly op: MapEditOp }
  | { readonly type: "list"; readonly path: Path; readonly ops: readonly ListEditOp[] }
  | { readonly type: "text"; readonly path: Path; readonly ops: readonly TextEditOp[] }
  | { readonly type: "richtext"; readonly path: Path; readonly ops: readonly RichTextEditOp[] }

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
 * of truth, asserted by the golden fixtures). The trailing codes are produced
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

const valueFinalizer = new FinalizationRegistry<WasmValueHandle>(handle => handle.free())
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

export function text(value: string): Text {
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

type NormalizedRichSpan =
  | { type: "text"; text: string; attrs: AttrEntry[] }
  | { type: "embed"; value: Value; attrs: AttrEntry[] }

function normalizedRichSpans(
  spans: readonly RichTextSpan[],
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
      result.push({ type: "embed", value: valueEntry[1] as Value, attrs })
    } else {
      throw new CollaError("invalid_value", operation, { reason: "unknown RichText span type" })
    }
  }
  if (limits !== undefined && result.length > limits.maxContainerLength) {
    limitExceeded("container length", result.length, limits.maxContainerLength, operation)
  }
  return result
}

export function richText(spans: readonly RichTextSpan[]): RichText {
  const normalized = normalizedRichSpans(spans, "rich_text")
  const frozen = normalized.map(span => {
    const attrs = attrsData(span.attrs)
    return Object.freeze(span.type === "text"
      ? { type: "text" as const, text: span.text, ...(attrs === undefined ? {} : { attrs }) }
      : { type: "embed" as const, value: span.value, ...(attrs === undefined ? {} : { attrs }) })
  })
  return Object.freeze({ type: "richtext", spans: Object.freeze(frozen) })
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

function compareUtf8(left: string, right: string): number {
  const a = utf8.encode(left)
  const b = utf8.encode(right)
  const length = Math.min(a.length, b.length)
  for (let index = 0; index < length; index += 1) {
    if (a[index] !== b[index]) return a[index] - b[index]
  }
  return a.length - b.length
}

function validateValue(input: Value, operation: string): void {
  const active = new WeakSet<object>()

  const visit = (value: Value): void => {
    if (value === null || typeof value === "boolean") {
      return
    }
    if (typeof value === "bigint") {
      if (value < I64_MIN || value > I64_MAX) {
        throw new CollaError("invalid_value", operation, {
          reason: "integer is outside the signed 64-bit range",
        })
      }
      return
    }
    if (typeof value === "number") {
      if (!Number.isFinite(value)) {
        throw new CollaError("invalid_value", operation, { reason: "number must be finite" })
      }
      return
    }
    if (typeof value === "string") {
      assertWellFormedString(value, operation)
      return
    }
    if (Array.isArray(value)) {
      if (active.has(value)) {
        throw new CollaError("invalid_value", operation, { reason: "cyclic Value" })
      }
      const values = ownArrayDataValues(value, operation)
      active.add(value)
      try {
        for (const child of values) visit(child as Value)
      } finally {
        active.delete(value)
      }
      return
    }
    if (isRecord(value)) {
      const entries = [...ownDataEntries(value, operation)]
      const marker = entries.find(([key]) => key === "type")
      if (marker?.[1] === "text") {
        if (entries.length !== 2 || !entries.some(([key]) => key === "value")) {
          throw new CollaError("invalid_value", operation, { reason: "invalid Text marker" })
        }
        const textValue = entries.find(([key]) => key === "value")?.[1]
        if (typeof textValue !== "string") {
          throw new CollaError("invalid_value", operation, { reason: "Text value must be a string" })
        }
        assertWellFormedString(textValue, operation)
        return
      }
      if (marker?.[1] === "richtext") {
        if (entries.length !== 2 || !entries.some(([key]) => key === "spans")) {
          throw new CollaError("invalid_value", operation, { reason: "invalid RichText marker" })
        }
        if (active.has(value)) {
          throw new CollaError("invalid_value", operation, { reason: "cyclic Value" })
        }
        active.add(value)
        try {
          const spans = normalizedRichSpans(
            entries.find(([key]) => key === "spans")?.[1] as readonly RichTextSpan[],
            operation,
          )
          for (const span of spans) {
            if (span.type === "embed") visit(span.value)
          }
        } finally {
          active.delete(value)
        }
        return
      }
      if (active.has(value)) {
        throw new CollaError("invalid_value", operation, { reason: "cyclic Value" })
      }
      for (const [key] of entries) assertWellFormedString(key, operation)
      active.add(value)
      try {
        for (const [, child] of entries) visit(child as Value)
      } finally {
        active.delete(value)
      }
      return
    }
    throw new CollaError("invalid_value", operation, { reason: "unsupported Value" })
  }

  visit(input)
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

function validateChangeAttrPatch(input: unknown, operation: string): void {
  if (input === undefined) return
  if (!isRecord(input)) throw invalidArgument(operation, "patch", "expected a plain record")
  for (const [key, value] of ownDataEntries(input, operation)) {
    assertWellFormedString(key, operation)
    const fields = changeInputFields(value, ["type", "value"], ["type"], operation, `patch.${key}`)
    const type = fields.get("type")
    if (type === "remove") {
      if (fields.has("value")) throw invalidArgument(operation, `patch.${key}`, "remove must not include value")
    } else if (type === "set" && fields.has("value")) {
      const record = Object.create(null) as Record<string, AttrValueData>
      record[key] = fields.get("value") as AttrValueData
      attrEntries(record, operation)
    } else {
      throw invalidArgument(operation, `patch.${key}`, "expected set with value or remove")
    }
  }
}

function validateChangeInput(input: ChangeInput, operation: string): void {
  const active = new WeakSet<object>()

  const visit = (value: unknown, context: string): void => {
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
          break
        }
        case "replace": {
          if (fields.size !== 2 || !fields.has("value")) {
            throw invalidArgument(operation, context, "replace requires value")
          }
          validateValue(fields.get("value") as Value, operation)
          break
        }
        case "map": {
          if (fields.size !== 2 || !fields.has("entries")) {
            throw invalidArgument(operation, context, "map requires entries")
          }
          const entries = changeInputArray(fields.get("entries"), operation, `${context}.entries`)
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
            if (item.get("type") === "insert" && item.size === 3 && item.has("value")) {
              validateValue(item.get("value") as Value, operation)
            } else if (item.get("type") === "delete" && item.size === 2) {
              // structurally valid
            } else if (item.get("type") === "modify" && item.size === 3 && item.has("change")) {
              visit(item.get("change"), `${entryContext}.change`)
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
              changeInputLength(item.get("length"), operation, `${opContext}.length`)
            } else if (item.get("type") === "insert" && item.size === 2 && item.has("values")) {
              const values = changeInputArray(item.get("values"), operation, `${opContext}.values`)
              values.forEach(value => validateValue(value as Value, operation))
            } else if (item.get("type") === "delete" && item.size === 2 && item.has("length")) {
              changeInputLength(item.get("length"), operation, `${opContext}.length`)
            } else if (item.get("type") === "modify" && item.size === 2 && item.has("change")) {
              visit(item.get("change"), `${opContext}.change`)
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
          ops.forEach((op, index) => {
            const opContext = `${context}.ops[${index}]`
            const item = changeInputFields(op, ["type", "length", "text"], ["type"], operation, opContext)
            if (item.get("type") === "retain" && item.size === 2 && item.has("length")) {
              changeInputLength(item.get("length"), operation, `${opContext}.length`)
            } else if (item.get("type") === "insert" && item.size === 2 && item.has("text")) {
              const text = item.get("text")
              if (typeof text !== "string") throw invalidArgument(operation, `${opContext}.text`, "expected a string")
              assertWellFormedString(text, operation)
            } else if (item.get("type") === "delete" && item.size === 2 && item.has("length")) {
              changeInputLength(item.get("length"), operation, `${opContext}.length`)
            } else {
              throw invalidArgument(operation, opContext, "invalid text operation")
            }
          })
          break
        }
        case "richtext": {
          if (fields.size !== 2 || !fields.has("ops")) {
            throw invalidArgument(operation, context, "richtext requires ops")
          }
          const ops = changeInputArray(fields.get("ops"), operation, `${context}.ops`)
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
              changeInputLength(item.get("length"), operation, `${opContext}.length`)
              validateChangeAttrPatch(item.get("patch"), operation)
            } else if (item.get("type") === "insert" && item.size === 2 && item.has("content")) {
              const contentContext = `${opContext}.content`
              const content = changeInputFields(
                item.get("content"),
                ["type", "text", "value", "attrs"],
                ["type"],
                operation,
                contentContext,
              )
              if (
                content.get("type") === "text" &&
                content.has("text") &&
                !content.has("value") &&
                content.size <= 3
              ) {
                const text = content.get("text")
                if (typeof text !== "string") throw invalidArgument(operation, `${contentContext}.text`, "expected a string")
                assertWellFormedString(text, operation)
              } else if (
                content.get("type") === "embed" &&
                content.has("value") &&
                !content.has("text") &&
                content.size <= 3
              ) {
                validateValue(content.get("value") as Value, operation)
              } else {
                throw invalidArgument(operation, contentContext, "invalid RichText content")
              }
              attrEntries(content.get("attrs") as AttrsData | undefined, operation)
            } else if (item.get("type") === "delete" && item.size === 2 && item.has("length")) {
              changeInputLength(item.get("length"), operation, `${opContext}.length`)
            } else {
              throw invalidArgument(operation, opContext, "invalid richtext operation")
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
          break
        }
        default:
          throw invalidArgument(operation, `${context}.type`, "unknown ChangeInput type")
      }
    } finally {
      active.delete(record)
    }
  }

  visit(input, "change")
}

function valueFromBytes(bytes: Uint8Array): Value {
  const handle = WasmValueHandle.decode(bytes)
  try {
    return handle.toJs() as Value
  } finally {
    handle.free()
  }
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

export class ValueHandle {
  #handle: WasmValueHandle | undefined

  private constructor(handle: WasmValueHandle) {
    this.#handle = handle
    valueFinalizer.register(this, handle, this)
  }

  static fromJS(input: Value, options?: InputOptions): ValueHandle {
    try {
      const limits = normalizeInputLimits(options, "value_from_js")
      validateValue(input, "value_from_js")
      return new ValueHandle(WasmValueHandle.fromJs(input, JSON.stringify(limits)))
    } catch (error) {
      throw fromWasmError(error, "value_from_js")
    }
  }

  static decode(bytes: Uint8Array): ValueHandle {
    if (!(bytes instanceof Uint8Array)) {
      throw invalidArgument("value_decode", "bytes", "expected Uint8Array")
    }
    try {
      return new ValueHandle(WasmValueHandle.decode(bytes))
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

  get(path: Path): Value {
    try {
      const bytes = this.#get("value_get").getBytes(pathJson(path, "value_get"))
      return valueFromBytes(new Uint8Array(bytes))
    } catch (error) {
      throw fromWasmError(error, "value_get", path)
    }
  }

  toJS(): Value {
    return this.#get("value_to_js").toJs() as Value
  }

  encode(): Uint8Array {
    return new Uint8Array(this.#get("value_encode").encode())
  }

  clone(): ValueHandle {
    return new ValueHandle(this.#get("value_clone").cloneHandle())
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

  #get(operation: string): WasmValueHandle {
    if (this.#handle === undefined) {
      throw invalidState(operation, "ValueHandle", "disposed")
    }
    return this.#handle
  }

  /** @internal */
  static _handle(value: ValueHandle, operation: string): WasmValueHandle {
    if (!(value instanceof ValueHandle)) {
      throw invalidArgument(operation, "value", "expected ValueHandle")
    }
    return value.#get(operation)
  }

  /** @internal */
  static _fromHandle(handle: WasmValueHandle): ValueHandle {
    return new ValueHandle(handle)
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
      validateChangeInput(input, "change_from_js")
      return new Change(ChangeHandle.fromJs(input, JSON.stringify(limits)))
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

  static decode(bytes: Uint8Array): Change {
    if (!(bytes instanceof Uint8Array)) {
      throw invalidArgument("change_decode", "bytes", "expected Uint8Array")
    }
    try {
      return new Change(ChangeHandle.decode(bytes))
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
  | (ChangeViewEntryBase & { readonly type: "value.replace"; readonly value: Value })
  | (ChangeViewEntryBase & { readonly type: "int.add"; readonly delta: bigint })
  | (ChangeViewEntryBase & { readonly type: "map.set"; readonly key: string; readonly value: Value })
  | (ChangeViewEntryBase & { readonly type: "map.delete"; readonly key: string })
  | (ChangeViewEntryBase & { readonly type: "list.insert"; readonly index: number; readonly values: readonly Value[] })
  | (ChangeViewEntryBase & { readonly type: "list.set"; readonly index: number; readonly value: Value })
  | (ChangeViewEntryBase & { readonly type: "list.delete"; readonly range: Range })
  | (ChangeViewEntryBase & { readonly type: "text.insert"; readonly at: number; readonly text: string })
  | (ChangeViewEntryBase & { readonly type: "text.delete"; readonly range: Range })
  | (ChangeViewEntryBase & { readonly type: "richtext.insertText"; readonly at: number; readonly text: string; readonly attrs?: AttrsData })
  | (ChangeViewEntryBase & { readonly type: "richtext.insertEmbed"; readonly at: number; readonly embed: Value; readonly attrs?: AttrsData })
  | (ChangeViewEntryBase & { readonly type: "richtext.delete"; readonly range: Range })
  | (ChangeViewEntryBase & { readonly type: "richtext.format"; readonly range: Range; readonly patch: AttrPatchView })

export type ChangeView = readonly ChangeViewEntry[]

export interface ChangeBuilder {
  noop(): this
  replace(value: Value): this
  map(edit: (map: MapChangeBuilder) => unknown): this
  list(edit: (list: ListChangeBuilder) => unknown): this
  text(edit: (text: TextChangeBuilder) => unknown): this
  richText(edit: (richText: RichTextChangeBuilder) => unknown): this
  intAdd(delta: bigint): this
}

export interface MapChangeBuilder {
  insert(key: string, value: Value): this
  delete(key: string): this
  modify(key: string, edit: (change: ChangeBuilder) => unknown): this
}

export interface ListChangeBuilder {
  retain(length: number): this
  insert(values: readonly Value[]): this
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
  insertEmbed(value: Value, attrs?: AttrsData): this
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

  replace(value: Value): this {
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
    return this.#select(Object.freeze({ type: "richtext", ops }))
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

  insert(key: string, value: Value): this {
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

  insert(values: readonly Value[]): this {
    this.assertActive()
    if (!Array.isArray(values)) {
      throw invalidArgument("change_build", "values", "expected an array")
    }
    this.#ops.push(Object.freeze({
      type: "insert",
      values: Object.freeze([...ownArrayDataValues(values, "change_build")]) as readonly Value[],
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

  insertEmbed(value: Value, attrs?: AttrsData): this {
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

export function apply(base: ValueHandle, change: Change): ValueHandle {
  try {
    return ValueHandle._fromHandle(
      applyHandles(
        ValueHandle._handle(base, "apply"),
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

export function invert(change: Change, base: ValueHandle): Change {
  try {
    return Change._fromHandle(invertHandle(
      Change._handle(change, "invert"),
      ValueHandle._handle(base, "invert"),
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

function viewValue(bytes: number[] | undefined): Value {
  return valueFromBytes(Uint8Array.from(bytes ?? []))
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

type RawMapEditOp =
  | { type: "insert"; valueBytes: number[] }
  | { type: "delete" }

type RawListEditOp =
  | { type: "retain"; length: number }
  | { type: "insert"; valuesBytes: number[][] }
  | { type: "delete"; length: number }
  | { type: "modify"; steps: RawEditStep[] }

type RawTextEditOp =
  | { type: "retain"; length: number }
  | { type: "insert"; text: string }
  | { type: "delete"; length: number }

type RawRichTextSpan =
  | { type: "text"; text: string; attrs?: AttrEntry[] }
  | { type: "embed"; valueBytes: number[]; attrs?: AttrEntry[] }

type RawRichTextEditOp =
  | { type: "retain"; length: number; patch?: RawChangeViewEntry["patch"] }
  | { type: "insert"; span: RawRichTextSpan }
  | { type: "delete"; length: number }

type RawEditStep =
  | { type: "replace"; path: (string | number)[]; valueBytes: number[] }
  | { type: "int"; path: (string | number)[]; delta: string }
  | { type: "map"; path: (string | number)[]; op: RawMapEditOp }
  | { type: "list"; path: (string | number)[]; ops: RawListEditOp[] }
  | { type: "text"; path: (string | number)[]; ops: RawTextEditOp[] }
  | { type: "richtext"; path: (string | number)[]; ops: RawRichTextEditOp[] }

function editStepsFromRaw(raw: readonly RawEditStep[]): readonly EditStep[] {
  const steps = raw.map((step): EditStep => {
    const path = [...step.path]
    switch (step.type) {
      case "replace":
        return { type: step.type, path, value: viewValue(step.valueBytes) }
      case "int":
        return { type: step.type, path, delta: BigInt(step.delta) }
      case "map":
        return {
          type: step.type,
          path,
          op: step.op.type === "delete"
            ? { type: "delete" }
            : { type: "insert", value: viewValue(step.op.valueBytes) },
        }
      case "list":
        return {
          type: step.type,
          path,
          ops: step.ops.map((op): ListEditOp => {
            switch (op.type) {
              case "retain": return { type: op.type, length: op.length }
              case "insert": return { type: op.type, values: op.valuesBytes.map(viewValue) }
              case "delete": return { type: op.type, length: op.length }
              case "modify": return { type: op.type, steps: editStepsFromRaw(op.steps) }
            }
          }),
        }
      case "text":
        return {
          type: step.type,
          path,
          ops: step.ops.map((op): TextEditOp => op.type === "insert"
            ? { type: op.type, text: op.text }
            : { type: op.type, length: op.length }),
        }
      case "richtext":
        return {
          type: step.type,
          path,
          ops: step.ops.map((op): RichTextEditOp => {
            if (op.type === "retain") {
              return op.patch === undefined
                ? { type: op.type, length: op.length }
                : { type: op.type, length: op.length, patch: viewPatch(op.patch) }
            }
            if (op.type === "delete") return { type: op.type, length: op.length }
            const attrs = viewAttrs(op.span.attrs)
            const span: RichTextSpan = op.span.type === "text"
              ? {
                  type: "text",
                  text: op.span.text,
                  ...(attrs === undefined ? {} : { attrs }),
                }
              : {
                  type: "embed",
                  value: viewValue(op.span.valueBytes),
                  ...(attrs === undefined ? {} : { attrs }),
                }
            return { type: op.type, span }
          }),
        }
    }
  })
  return deepFreeze(steps)
}

export function convertChangeToEditSteps(
  change: Change,
  base: ValueHandle,
): readonly EditStep[] {
  const operation = "convert_change_to_edit_steps"
  try {
    const raw = JSON.parse(convertChangeToEditStepsHandle(
      Change._handle(change, operation),
      ValueHandle._handle(base, operation),
    )) as RawEditStep[]
    return editStepsFromRaw(raw)
  } catch (error) {
    throw fromWasmError(error, operation)
  }
}

export function inspectChange(change: Change, base: ValueHandle): ChangeView {
  const operation = "inspect_change"
  try {
    const raw = JSON.parse(inspectChangeHandle(
      Change._handle(change, operation),
      ValueHandle._handle(base, operation),
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
        case "richtext.insertText": {
          const attrs = viewAttrs(entry.attrs)
          return Object.freeze({
            type: entry.type,
            path,
            at: entry.at ?? 0,
            text: entry.text ?? "",
            ...(attrs === undefined ? {} : { attrs }),
          })
        }
        case "richtext.insertEmbed": {
          const attrs = viewAttrs(entry.attrs)
          return Object.freeze({
            type: entry.type,
            path,
            at: entry.at ?? 0,
            embed: viewValue(entry.embedBytes),
            ...(attrs === undefined ? {} : { attrs }),
          })
        }
        case "richtext.delete": return Object.freeze({ type: entry.type, path, range: viewRange(entry) })
        case "richtext.format": return Object.freeze({
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
  value: ValueHandle,
  path: Path,
  utf16Position: number,
): number {
  const operation = "resolve_code_point_position"
  const position = indexArgument(utf16Position, operation, "utf16Position")
  try {
    return resolveCodePointPositionHandle(
      ValueHandle._handle(value, operation),
      pathJson(path, operation),
      position,
    )
  } catch (error) {
    throw fromWasmError(error, operation, path)
  }
}

export function resolveUtf16Position(
  value: ValueHandle,
  path: Path,
  codePointPosition: number,
): number {
  const operation = "resolve_utf16_position"
  const position = indexArgument(codePointPosition, operation, "codePointPosition")
  try {
    return resolveUtf16PositionHandle(
      ValueHandle._handle(value, operation),
      pathJson(path, operation),
      position,
    )
  } catch (error) {
    throw fromWasmError(error, operation, path)
  }
}
