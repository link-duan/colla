import {
  applyHandles,
  BuilderHandle,
  ChangeHandle,
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
  | readonly ValueInput[]
  | ValueInputMap
export interface ValueInputMap {
  readonly [key: string]: ValueInput
}
export type ValueData =
  | null
  | boolean
  | bigint
  | number
  | string
  | readonly ValueData[]
  | ValueDataMap
export interface ValueDataMap {
  readonly [key: string]: ValueData
}

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
  readonly code: string
  readonly operation: string
  readonly path?: Path
  readonly details: Readonly<Record<string, unknown>>

  constructor(
    code: string,
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

  is(code: string): boolean {
    return this.code === code
  }
}

function invalidArgument(operation: string, argument: string, reason: string): CollaError {
  return new CollaError("invalid_argument", operation, { argument, reason })
}

function invalidState(
  operation: string,
  resource: string,
  reason: "disposed" | "consumed",
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
    payload.code ?? "invalid_argument",
    payload.operation ?? fallbackOperation,
    payload.details ?? { reason: String(error) },
    path,
  )
}

const valueFinalizer = new FinalizationRegistry<ValueHandle>(handle => handle.free())
const changeFinalizer = new FinalizationRegistry<ChangeHandle>(handle => handle.free())
const builderFinalizer = new FinalizationRegistry<BuilderHandle>(handle => handle.free())

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
      check("string bytes", utf8.encode(value).length, limits?.maxStringBytes)
      writer.byte(5)
      writer.string(value)
    } else if (Array.isArray(value)) {
      if (active.has(value)) {
        throw new CollaError("invalid_value", operation, { reason: "cyclic ValueInput" })
      }
      check("container length", value.length, limits?.maxContainerLength)
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
      }
      for (let index = 0; index < value.length; index += 1) {
        if (!Object.hasOwn(value, index)) {
          throw new CollaError("invalid_value", operation, { reason: "sparse arrays are not supported" })
        }
      }
      active.add(value)
      try {
        writer.byte(8)
        writer.varint(BigInt(value.length))
        for (let index = 0; index < value.length; index += 1) encode(value[index], depth + 1)
      } finally {
        active.delete(value)
      }
    } else if (isRecord(value)) {
      if (active.has(value)) {
        throw new CollaError("invalid_value", operation, { reason: "cyclic ValueInput" })
      }
      const entries = [...ownDataEntries(value, operation)].sort(([left], [right]) => compareUtf8(left, right))
      check("container length", entries.length, limits?.maxContainerLength)
      for (const [key] of entries) {
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

  change(): ChangeBuilder {
    return ChangeBuilder._fromHandle(this.#get("value_change").change())
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

  /** @internal */
  static _fromTrustedJS(input: ValueInput, operation: string): Value {
    const bytes = encodeValueInput(input, operation)
    return new Value(ValueHandle.decodeTrusted(bytes))
  }
}

export class Change {
  #handle: ChangeHandle | undefined

  private constructor(handle: ChangeHandle) {
    this.#handle = handle
    changeFinalizer.register(this, handle, this)
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

export class ChangeBuilder {
  #handle: BuilderHandle | undefined
  #consumed = false

  private constructor(handle: BuilderHandle) {
    this.#handle = handle
    builderFinalizer.register(this, handle, this)
  }

  replace(path: Path, value: ValueInput): this {
    const handle = this.#get("builder_replace")
    if (!Array.isArray(path) || path.length !== 0) {
      throw invalidArgument("builder_replace", "path", "minimal tracer supports root path only")
    }
    const replacement = Value._fromTrustedJS(value, "builder_replace")
    try {
      handle.replaceRoot(Value._handle(replacement, "builder_replace"))
      return this
    } catch (error) {
      throw fromWasmError(error, "builder_replace")
    } finally {
      replacement.dispose()
    }
  }

  build(): Change {
    const handle = this.#get("builder_build")
    try {
      const change = Change._fromHandle(handle.build())
      this.#consume()
      return change
    } catch (error) {
      throw fromWasmError(error, "builder_build")
    }
  }

  dispose(): void {
    const handle = this.#handle
    if (handle === undefined) return
    this.#handle = undefined
    builderFinalizer.unregister(this)
    handle.free()
  }

  [Symbol.dispose](): void {
    this.dispose()
  }

  #consume(): void {
    const handle = this.#handle
    if (handle === undefined) return
    this.#handle = undefined
    this.#consumed = true
    builderFinalizer.unregister(this)
    handle.free()
  }

  #get(operation: string): BuilderHandle {
    if (this.#handle === undefined) {
      throw invalidState(operation, "ChangeBuilder", this.#consumed ? "consumed" : "disposed")
    }
    return this.#handle
  }

  /** @internal */
  static _fromHandle(handle: BuilderHandle): ChangeBuilder {
    return new ChangeBuilder(handle)
  }
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
