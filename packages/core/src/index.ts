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

export type ValueInput = null | boolean | bigint | number | string
export type ValueData = ValueInput

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

function fromWasmError(error: unknown, fallbackOperation: string): CollaError {
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

function valueHandleFromJS(input: ValueInput): ValueHandle {
  if (input === null) return ValueHandle.null()
  if (typeof input === "boolean") return ValueHandle.bool(input)
  if (typeof input === "bigint") return ValueHandle.int(int(input))
  if (typeof input === "number") {
    if (!Number.isFinite(input)) {
      throw new CollaError("invalid_value", "value_from_js", {
        reason: "number must be finite",
      })
    }
    return ValueHandle.float(input)
  }
  if (typeof input === "string") return ValueHandle.string(input)
  throw new CollaError("invalid_value", "value_from_js", {
    reason: "unsupported ValueInput",
  })
}

export class Value {
  #handle: ValueHandle | undefined

  private constructor(handle: ValueHandle) {
    this.#handle = handle
    valueFinalizer.register(this, handle, this)
  }

  static fromJS(input: ValueInput): Value {
    try {
      return new Value(valueHandleFromJS(input))
    } catch (error) {
      throw fromWasmError(error, "value_from_js")
    }
  }

  static decode(bytes: Uint8Array): Value {
    if (!(bytes instanceof Uint8Array)) {
      throw invalidArgument("value_decode", "bytes", "expected Uint8Array")
    }
    try {
      return new Value(ValueHandle.decode(bytes))
    } catch (error) {
      throw fromWasmError(error, "value_decode")
    }
  }

  kind(path: Path = []): ValueKind {
    if (path.length !== 0) {
      throw invalidArgument("value_kind", "path", "nested paths are not available yet")
    }
    return this.#get("value_kind").kind() as ValueKind
  }

  toJS(): ValueData {
    const handle = this.#get("value_to_js")
    switch (handle.kind() as ValueKind) {
      case "null": return null
      case "bool": return handle.boolValue()
      case "int": return handle.intValue()
      case "float": return handle.floatValue()
      case "string": return handle.stringValue()
      default:
        throw new CollaError("invalid_state", "value_to_js", {
          resource: "Value",
          reason: "unsupported_kind",
        })
    }
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
}

export class Change {
  #handle: ChangeHandle | undefined

  private constructor(handle: ChangeHandle) {
    this.#handle = handle
    changeFinalizer.register(this, handle, this)
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
    const replacement = Value.fromJS(value)
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
