import {
  decodeSnapshotEnvelope,
  decodeSnapshotValue,
  decodeUpdateChange,
  decodeUpdateEnvelope,
  encodeSnapshot,
  encodeUpdate,
} from "./internal/colla_wasm.js"
import {
  apply,
  buildListOps,
  buildTextOps,
  Change,
  CollaError,
  compose,
  convertChangeToEditSteps,
  resolveCodePointPosition,
  resolveUtf16Position,
  transformPair,
  ValueHandle,
  type ChangeInput,
  type EditStep,
  type InputOptions,
  type ListChangeOpInput,
  type ListOpStream,
  type Path,
  type TextChangeOpInput,
  type TextOpStream,
  type Value,
  type ValueKind,
} from "./index.js"

type WasmErrorPayload = {
  readonly code?: string
  readonly operation?: string
  readonly details?: Record<string, unknown>
}

const U64_MAX = (1n << 64n) - 1n

function revisionArgument(value: unknown, operation: string, argument: string): bigint {
  if (typeof value === "number" && Number.isSafeInteger(value) && value >= 0) {
    return BigInt(value)
  }
  if (typeof value !== "bigint" || value < 0n || value > U64_MAX) {
    throw new CollaError("invalid_argument", operation, {
      argument,
      reason: "expected an unsigned 64-bit bigint",
    })
  }
  return value
}

function revisionOverflow(operation: string): CollaError {
  return new CollaError("limit_exceeded", operation, { reason: "revision_overflow" })
}

function updateIdOverflow(operation: string): CollaError {
  return new CollaError("limit_exceeded", operation, { reason: "update_id_overflow" })
}

function fromWasmError(error: unknown, operation: string): CollaError {
  if (error instanceof CollaError) return error
  try {
    const payload = JSON.parse(String(error)) as WasmErrorPayload
    if (payload !== null && typeof payload === "object" && typeof payload.code === "string") {
      return new CollaError(
        payload.code as ConstructorParameters<typeof CollaError>[0],
        payload.operation ?? operation,
        payload.details ?? { reason: String(error) },
      )
    }
  } catch {
    // Fall through to a stable local error below.
  }
  return new CollaError("invalid_argument", operation, { reason: String(error) })
}

/** A persistable content snapshot containing a revision, pure Core Value, and binary envelope. */
export interface Snapshot {
  readonly revision: bigint
  readonly bytes: Uint8Array
  readonly value: Value
}

export const Snapshot = {
  /** Decodes a Snapshot from a binary snapshot envelope. */
  decode(bytes: Uint8Array): Snapshot {
    if (!(bytes instanceof Uint8Array)) {
      throw new CollaError("invalid_argument", "snapshot_decode", {
        argument: "bytes",
        reason: "expected Uint8Array",
      })
    }
    try {
      const envelope = decodeSnapshotEnvelope(bytes) as { revision: bigint; value: Value }
      return Object.freeze({
        revision: envelope.revision,
        bytes: new Uint8Array(bytes),
        value: envelope.value,
      })
    } catch (error) {
      throw fromWasmError(error, "snapshot_decode")
    }
  },

  /** Creates a Snapshot from a Core Value handle. */
  fromValue(value: ValueHandle, revision: number | bigint = 0n): Snapshot {
    const operation = "snapshot_from_value"
    const rev = revisionArgument(revision, operation, "revision")
    try {
      const handle = ValueHandle._handle(value, operation)
      const bytes = new Uint8Array(encodeSnapshot(rev, handle))
      const val = value.toJS()
      return Object.freeze({
        revision: rev,
        bytes,
        value: val,
      })
    } catch (error) {
      if (error instanceof CollaError) throw error
      throw new CollaError("invalid_argument", operation, { reason: String(error) })
    }
  },

  /** Creates a Snapshot from structured Core Value input. */
  fromJS(
    input: Parameters<typeof ValueHandle.fromJS>[0],
    revision: number | bigint = 0n,
    options?: InputOptions,
  ): Snapshot {
    const value = ValueHandle.fromJS(input, options)
    try {
      return Snapshot.fromValue(value, revision)
    } finally {
      value.dispose()
    }
  },
}

/** A versioned Core Change exchanged by a Document. */
export interface Update {
  readonly revision: bigint
  readonly updateId: bigint
  readonly bytes: Uint8Array
}

export const Update = {
  /** Decodes an Update from a binary update envelope. */
  decode(bytes: Uint8Array): Update {
    if (!(bytes instanceof Uint8Array)) {
      throw new CollaError("invalid_argument", "update_decode", {
        argument: "bytes",
        reason: "expected Uint8Array",
      })
    }
    try {
      const envelope = decodeUpdateEnvelope(bytes) as { revision: bigint; updateId: bigint }
      return Object.freeze({
        revision: envelope.revision,
        updateId: envelope.updateId,
        bytes: new Uint8Array(bytes),
      })
    } catch (error) {
      throw fromWasmError(error, "update_decode")
    }
  },
}

/** @internal */
export function _createUpdate(revision: bigint, updateId: bigint, change: Change): Update {
  return createUpdate(revision, updateId, change)
}

function createUpdate(revision: bigint, updateId: bigint, change: Change): Update {
  const handle = Change._handle(change, "update_encode")
  const bytes = new Uint8Array(encodeUpdate(revision, updateId, handle))
  return Object.freeze({
    revision,
    updateId,
    bytes,
  })
}

/** A change event emitted when the visible Document content changes. */
export interface DocumentChangeEvent {
  readonly origin: "local" | "remote"
  readonly editSteps: readonly EditStep[]
  readonly revision: bigint
}

export type DocumentChangeSubscriber = (event: DocumentChangeEvent) => unknown
export type DocumentErrorSubscriber = (error: unknown) => unknown

export type DocumentSubscriber =
  | DocumentChangeSubscriber
  | {
      readonly onChange: DocumentChangeSubscriber
      readonly onError?: DocumentErrorSubscriber
    }

type SubscriberEntry = {
  readonly onChange: DocumentChangeSubscriber
  readonly onError?: DocumentErrorSubscriber
}

type Pending = {
  readonly updateId: bigint
  readonly change: Change
}

/** Transactional mutation scope for atomic Document updates. */
export interface TransactionContext {
  /** Replaces or sets a value at the given path (auto-upserts in maps). */
  set(path: Path, value: unknown): void
  /** Deletes an entry or element at the given path. */
  delete(path: Path): void
  /** Mutates a collaborative text at the given path using single-pass streaming ops. */
  text(
    path: Path,
    editOrOps: ((stream: TextOpStream) => unknown) | readonly TextChangeOpInput[],
  ): void
  /** Mutates a list at the given path using single-pass streaming ops. */
  list(
    path: Path,
    editOrOps: ((stream: ListOpStream) => unknown) | readonly ListChangeOpInput[],
  ): void
}

function wrapPathChange(
  workingValue: ValueHandle,
  path: Path,
  operation: string,
  leafChange: (targetKind: ValueKind) => ChangeInput,
): ChangeInput {
  if (!Array.isArray(path)) {
    throw new CollaError("invalid_argument", operation, {
      argument: "path",
      reason: "expected an array",
    })
  }
  if (path.length === 0) {
    const rootKind = workingValue.kind([])
    return leafChange(rootKind)
  }

  const parentPath = path.slice(0, -1)
  const lastSegment = path[path.length - 1]
  const parentKind = workingValue.kind(parentPath)

  let currentInput: ChangeInput

  if (parentKind === "map") {
    if (typeof lastSegment !== "string") {
      throw new CollaError("invalid_argument", operation, {
        path,
        reason: `expected string key for map at ${JSON.stringify(parentPath)}, got ${typeof lastSegment}`,
      })
    }
    const keyExists = workingValue.has(path)
    const targetKind = keyExists ? workingValue.kind(path) : "null"
    const innerChange = leafChange(targetKind)
    if (innerChange.type === "replace") {
      if (keyExists) {
        currentInput = {
          type: "map",
          entries: [{ key: lastSegment, type: "modify", change: innerChange }],
        }
      } else {
        currentInput = {
          type: "map",
          entries: [{ key: lastSegment, type: "insert", value: innerChange.value }],
        }
      }
    } else if (innerChange.type === "noop" && "deleteEntry" in innerChange) {
      currentInput = {
        type: "map",
        entries: [{ key: lastSegment, type: "delete" }],
      }
    } else {
      currentInput = {
        type: "map",
        entries: [{ key: lastSegment, type: "modify", change: innerChange }],
      }
    }
  } else if (parentKind === "list") {
    if (typeof lastSegment !== "number" || !Number.isSafeInteger(lastSegment) || lastSegment < 0) {
      throw new CollaError("invalid_argument", operation, {
        path,
        reason: `expected non-negative integer index for list at ${JSON.stringify(parentPath)}`,
      })
    }
    const indexExists = workingValue.has(path)
    const targetKind = indexExists ? workingValue.kind(path) : "null"
    const innerChange = leafChange(targetKind)
    if (innerChange.type === "replace") {
      const ops: ListChangeOpInput[] = []
      if (lastSegment > 0) ops.push({ type: "retain", length: lastSegment })
      if (indexExists) {
        ops.push({ type: "modify", change: innerChange })
      } else {
        ops.push({ type: "insert", values: [innerChange.value] })
      }
      currentInput = { type: "list", ops }
    } else if (innerChange.type === "noop" && "deleteEntry" in innerChange) {
      const ops: ListChangeOpInput[] = []
      if (lastSegment > 0) ops.push({ type: "retain", length: lastSegment })
      ops.push({ type: "delete", length: 1 })
      currentInput = { type: "list", ops }
    } else {
      const ops: ListChangeOpInput[] = []
      if (lastSegment > 0) ops.push({ type: "retain", length: lastSegment })
      ops.push({ type: "modify", change: innerChange })
      currentInput = { type: "list", ops }
    }
  } else {
    throw new CollaError("invalid_argument", operation, {
      path,
      reason: `cannot mutate child of non-container kind '${parentKind}' at ${JSON.stringify(parentPath)}`,
    })
  }

  for (let i = parentPath.length - 1; i >= 0; i--) {
    const segment = parentPath[i]
    const currentContainerPath = parentPath.slice(0, i)
    const containerKind = workingValue.kind(currentContainerPath)
    if (containerKind === "map") {
      if (typeof segment !== "string") {
        throw new CollaError("invalid_argument", operation, {
          path,
          reason: `expected string key for map at ${JSON.stringify(currentContainerPath)}`,
        })
      }
      currentInput = {
        type: "map",
        entries: [{ key: segment, type: "modify", change: currentInput }],
      }
    } else if (containerKind === "list") {
      if (typeof segment !== "number" || !Number.isSafeInteger(segment) || segment < 0) {
        throw new CollaError("invalid_argument", operation, {
          path,
          reason: `expected non-negative integer index for list at ${JSON.stringify(currentContainerPath)}`,
        })
      }
      const ops: ListChangeOpInput[] = []
      if (segment > 0) ops.push({ type: "retain", length: segment })
      ops.push({ type: "modify", change: currentInput })
      currentInput = { type: "list", ops }
    } else {
      throw new CollaError("invalid_argument", operation, {
        path,
        reason: `cannot traverse non-container kind '${containerKind}' at ${JSON.stringify(currentContainerPath)}`,
      })
    }
  }

  return currentInput
}

/** Mutable document state with local and remote update handling. */
export class Document {
  #value: ValueHandle
  #confirmedValue: ValueHandle
  #revision: bigint
  #confirmedRevision: bigint
  #nextUpdateId = 1n
  #pending: Pending[] = []
  #subscribers = new Set<SubscriberEntry>()
  #inTransaction = false
  #disposed = false

  private constructor(value: ValueHandle, revision: bigint) {
    this.#value = value
    this.#confirmedValue = value.clone()
    this.#revision = revision
    this.#confirmedRevision = revision
  }

  /** Restores a Document from a content Snapshot or binary snapshot bytes. */
  static fromSnapshot(snapshot: Snapshot | Uint8Array): Document {
    let valueHandle: ValueHandle
    let revision: bigint

    if (snapshot instanceof Uint8Array) {
      try {
        const envelope = decodeSnapshotEnvelope(snapshot) as { revision: bigint }
        revision = envelope.revision
        valueHandle = ValueHandle._fromHandle(decodeSnapshotValue(snapshot))
      } catch (error) {
        throw fromWasmError(error, "document_from_snapshot")
      }
    } else if (
      snapshot !== null &&
      typeof snapshot === "object" &&
      typeof snapshot.revision === "bigint"
    ) {
      revision = snapshot.revision
      if (snapshot.bytes instanceof Uint8Array) {
        try {
          valueHandle = ValueHandle._fromHandle(decodeSnapshotValue(snapshot.bytes))
        } catch (error) {
          throw fromWasmError(error, "document_from_snapshot")
        }
      } else {
        valueHandle = ValueHandle.fromJS(snapshot.value)
      }
    } else {
      throw new CollaError("invalid_argument", "document_from_snapshot", {
        argument: "snapshot",
        reason: "expected Snapshot or Uint8Array",
      })
    }

    return new Document(valueHandle, revision)
  }

  /** Creates a Document from structured Core Value input. */
  static fromJS(
    input: Parameters<typeof ValueHandle.fromJS>[0],
    revision: number | bigint = 0n,
    options?: InputOptions,
  ): Document {
    const rev = revisionArgument(revision, "document_from_js", "revision")
    const value = ValueHandle.fromJS(input, options)
    return new Document(value, rev)
  }

  get revision(): bigint {
    this.#assertActive("document_revision")
    return this.#revision
  }

  get confirmedRevision(): bigint {
    this.#assertActive("document_confirmed_revision")
    return this.#confirmedRevision
  }

  get hasPending(): boolean {
    this.#assertActive("document_has_pending")
    return this.#pending.length > 0
  }

  get pendingCount(): number {
    this.#assertActive("document_pending_count")
    return this.#pending.length
  }

  /** Returns an independently owned handle for the visible content. */
  value(): ValueHandle {
    this.#assertActive("document_value")
    return this.#value.clone()
  }

  /** Resolves a Snapshot-relative Path and borrows the target Value without cloning. */
  get(path: Path): Value {
    this.#assertActive("document_get")
    return this.#value.get(path)
  }

  /** Returns whether a Snapshot-relative Path exists. */
  has(path: Path): boolean {
    this.#assertActive("document_has")
    return this.#value.has(path)
  }

  /** Returns the ValueKind at a Snapshot-relative Path. */
  kind(path: Path = []): ValueKind {
    this.#assertActive("document_kind")
    return this.#value.kind(path)
  }

  /** Converts a UTF-16 position in visible Text/RichText to a code point position. */
  resolveCodePointPosition(path: Path, utf16Position: number): number {
    this.#assertActive("document_resolve_code_point_position")
    return resolveCodePointPosition(this.#value, path, utf16Position)
  }

  /** Converts a code point position in visible Text/RichText to a UTF-16 position. */
  resolveUtf16Position(path: Path, codePointPosition: number): number {
    this.#assertActive("document_resolve_utf16_position")
    return resolveUtf16Position(this.#value, path, codePointPosition)
  }

  /** Encodes the visible content and current revision as a pure Snapshot. */
  snapshot(): Snapshot {
    this.#assertActive("document_snapshot")
    return Snapshot.fromValue(this.#value, this.#revision)
  }

  /** Executes an atomic mutation transaction on the Document. */
  transact(fn: (tx: TransactionContext) => void): Update {
    this.#assertActive("document_transact")
    if (typeof fn !== "function") {
      throw new CollaError("invalid_argument", "document_transact", {
        argument: "fn",
        reason: "expected a function",
      })
    }
    if (this.#inTransaction) {
      throw new CollaError("invalid_state", "document_transact", {
        reason: "nested transactions are not supported",
      })
    }
    if (this.#revision === U64_MAX) throw revisionOverflow("document_transact")
    if (this.#nextUpdateId > U64_MAX) throw updateIdOverflow("document_transact")

    this.#inTransaction = true
    let workingValue = this.#value.clone()
    const changes: Change[] = []

    const applyStep = (changeInput: ChangeInput) => {
      const change = Change.fromJS(changeInput)
      let nextVal: ValueHandle
      try {
        nextVal = apply(workingValue, change)
      } catch (error) {
        change.dispose()
        throw error
      }
      workingValue.dispose()
      workingValue = nextVal
      changes.push(change)
    }

    const tx: TransactionContext = {
      set: (path, val) => {
        const changeInput = wrapPathChange(workingValue, path, "transaction_set", () => ({
          type: "replace",
          value: val as Value,
        }))
        applyStep(changeInput)
      },
      delete: (path) => {
        if (!Array.isArray(path) || path.length === 0) {
          throw new CollaError("invalid_argument", "transaction_delete", {
            path,
            reason: "cannot delete root document",
          })
        }
        const changeInput = wrapPathChange(workingValue, path, "transaction_delete", () => ({
          type: "noop",
          deleteEntry: true,
        } as unknown as ChangeInput))
        applyStep(changeInput)
      },
      text: (path, editOrOps) => {
        const currentVal = workingValue.get(path)
        let baseText: string | null = null
        if (typeof currentVal === "string") {
          baseText = currentVal
        } else if (
          currentVal !== null &&
          typeof currentVal === "object" &&
          "type" in currentVal &&
          currentVal.type === "text" &&
          typeof currentVal.value === "string"
        ) {
          baseText = currentVal.value
        }
        if (baseText === null) {
          throw new CollaError("invalid_argument", "transaction_text", {
            path,
            reason: `expected Text at path ${JSON.stringify(path)}`,
          })
        }
        const ops = buildTextOps(baseText, editOrOps)
        const changeInput = wrapPathChange(workingValue, path, "transaction_text", () => ({
          type: "text",
          ops,
        }))
        applyStep(changeInput)
      },
      list: (path, editOrOps) => {
        const currentVal = workingValue.get(path)
        if (!Array.isArray(currentVal)) {
          throw new CollaError("invalid_argument", "transaction_list", {
            path,
            reason: `expected List at path ${JSON.stringify(path)}`,
          })
        }
        const ops = buildListOps(editOrOps)
        const changeInput = wrapPathChange(workingValue, path, "transaction_list", () => ({
          type: "list",
          ops,
        }))
        applyStep(changeInput)
      },
    }

    let finalChange: Change
    try {
      fn(tx)
      if (changes.length === 0) {
        finalChange = Change.build((e) => e.noop())
      } else if (changes.length === 1) {
        finalChange = changes[0]
      } else {
        let composed = changes[0]
        for (let i = 1; i < changes.length; i++) {
          const next = changes[i]
          let nextComposed: Change
          try {
            nextComposed = compose(composed, next)
          } catch (err) {
            composed.dispose()
            for (let j = i; j < changes.length; j++) changes[j].dispose()
            throw err
          }
          composed.dispose()
          next.dispose()
          composed = nextComposed
        }
        finalChange = composed
      }
    } catch (error) {
      workingValue.dispose()
      for (const c of changes) c.dispose()
      this.#inTransaction = false
      throw error
    }

    const before = this.#value
    this.#value = workingValue
    const updateId = this.#nextUpdateId
    this.#nextUpdateId += 1n
    const revision = this.#revision
    this.#revision += 1n

    let update: Update
    try {
      update = createUpdate(revision, updateId, finalChange)
    } catch (error) {
      finalChange.dispose()
      before.dispose()
      this.#inTransaction = false
      throw error
    }

    this.#pending.push({ updateId, change: finalChange })
    this.#inTransaction = false

    try {
      this.#emitChange("local", finalChange, before)
    } finally {
      before.dispose()
    }

    return update
  }

  /** Applies a remote Update or binary update bytes and rebases local pending changes. */
  applyRemote(updateOrBytes: Update | Uint8Array): void {
    this.#assertActive("document_apply_remote")
    let updateBytes: Uint8Array
    let revision: bigint

    if (updateOrBytes instanceof Uint8Array) {
      updateBytes = updateOrBytes
      try {
        const header = decodeUpdateEnvelope(updateBytes) as { revision: bigint; updateId: bigint }
        revision = header.revision
      } catch (error) {
        throw fromWasmError(error, "document_apply_remote")
      }
    } else if (
      updateOrBytes !== null &&
      typeof updateOrBytes === "object" &&
      typeof updateOrBytes.revision === "bigint" &&
      updateOrBytes.bytes instanceof Uint8Array
    ) {
      updateBytes = updateOrBytes.bytes
      revision = updateOrBytes.revision
    } else {
      throw new CollaError("invalid_argument", "document_apply_remote", {
        argument: "update",
        reason: "expected Update or Uint8Array",
      })
    }

    if (revision !== this.#confirmedRevision) {
      throw new CollaError("incompatible_change", "document_apply_remote", {
        reason: "revision_mismatch",
        expected: this.#confirmedRevision,
        actual: revision,
      })
    }
    if (this.#confirmedRevision === U64_MAX || this.#revision === U64_MAX) {
      throw revisionOverflow("document_apply_remote")
    }

    let remoteChange: Change
    try {
      remoteChange = Change._fromHandle(decodeUpdateChange(updateBytes))
    } catch (error) {
      throw fromWasmError(error, "document_apply_remote")
    }

    const before = this.#value.clone()
    let visibleChange = remoteChange
    let confirmedNext: ValueHandle | undefined
    let visibleNext: ValueHandle | undefined
    const nextPending: Pending[] = []

    try {
      confirmedNext = apply(this.#confirmedValue, visibleChange)
      for (const pending of this.#pending) {
        const pendingId = pending.updateId
        const [pendingPrime, remotePrime] = transformPair(
          pending.change,
          visibleChange,
          { order: "left-first" },
        )
        visibleChange.dispose()
        visibleChange = remotePrime
        nextPending.push({
          updateId: pendingId,
          change: pendingPrime,
        })
      }
      visibleNext = apply(this.#value, visibleChange)
    } catch (error) {
      visibleChange.dispose()
      before.dispose()
      confirmedNext?.dispose()
      visibleNext?.dispose()
      for (const p of nextPending) {
        p.change.dispose()
      }
      throw error
    }

    for (const pending of this.#pending) {
      pending.change.dispose()
    }
    this.#confirmedValue.dispose()
    this.#confirmedValue = confirmedNext
    this.#value.dispose()
    this.#value = visibleNext
    this.#confirmedRevision += 1n
    this.#revision += 1n
    this.#pending = nextPending

    try {
      this.#emitChange("remote", visibleChange, before)
    } finally {
      visibleChange.dispose()
      before.dispose()
    }
  }

  /** Acknowledges pending local updates up to updateId (cumulative ACK). */
  ack(updateId: number | bigint): void {
    this.#assertActive("document_ack")
    const id = revisionArgument(updateId, "document_ack", "updateId")

    const targetIndex = this.#pending.findIndex((p) => p.updateId === id)
    if (targetIndex === -1) {
      throw new CollaError("invalid_argument", "document_ack", {
        argument: "updateId",
        reason: "unknown_or_out_of_order_update",
      })
    }

    for (let i = 0; i <= targetIndex; i++) {
      if (this.#confirmedRevision === U64_MAX) throw revisionOverflow("document_ack")
      const item = this.#pending[i]
      const confirmedNext = apply(this.#confirmedValue, item.change)
      this.#confirmedValue.dispose()
      this.#confirmedValue = confirmedNext
      this.#confirmedRevision += 1n
      item.change.dispose()
    }

    this.#pending = this.#pending.slice(targetIndex + 1)
  }

  /** Subscribes to Document change events and returns an unsubscribe closure. */
  subscribe(subscriber: DocumentSubscriber): () => void {
    this.#assertActive("document_subscribe")
    let changeSub: DocumentChangeSubscriber
    let errorSub: DocumentErrorSubscriber | undefined

    if (typeof subscriber === "function") {
      changeSub = subscriber
    } else if (
      subscriber !== null &&
      typeof subscriber === "object" &&
      typeof subscriber.onChange === "function"
    ) {
      changeSub = subscriber.onChange
      if (typeof subscriber.onError === "function") {
        errorSub = subscriber.onError
      }
    } else {
      throw new CollaError("invalid_argument", "document_subscribe", {
        argument: "subscriber",
        reason: "expected a subscriber function or an object with onChange",
      })
    }

    const entry: SubscriberEntry = { onChange: changeSub, onError: errorSub }
    this.#subscribers.add(entry)
    return () => {
      this.#subscribers.delete(entry)
    }
  }

  dispose(): void {
    if (this.#disposed) return
    this.#disposed = true
    this.#value.dispose()
    this.#confirmedValue.dispose()
    for (const pending of this.#pending) {
      pending.change.dispose()
    }
    this.#pending = []
    this.#subscribers.clear()
  }

  [Symbol.dispose](): void {
    this.dispose()
  }

  #emitChange(
    origin: DocumentChangeEvent["origin"],
    change: Change,
    before: ValueHandle,
  ): void {
    if (this.#subscribers.size === 0) return
    const editSteps = convertChangeToEditSteps(change, before)
    const event: DocumentChangeEvent = Object.freeze({
      origin,
      editSteps,
      revision: this.#revision,
    })
    for (const listener of [...this.#subscribers]) {
      try {
        listener.onChange(event)
      } catch (error) {
        if (listener.onError) {
          try {
            listener.onError(error)
          } catch {
            // Observer error subscriber failed; prevent recursion.
          }
        } else {
          console.error("[colla-ot] Uncaught error in Document change subscriber:", error)
        }
      }
    }
  }

  #assertActive(operation: string): void {
    if (this.#disposed) {
      throw new CollaError("invalid_state", operation, {
        resource: "Document",
        reason: "disposed",
      })
    }
  }
}
