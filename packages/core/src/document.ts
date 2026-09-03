import { SnapshotHandle, UpdateHandle } from "./internal/colla_wasm.js"
import {
  apply,
  Change,
  CollaError,
  convertChangeToEditSteps,
  transformPair,
  ValueHandle,
  type EditStep,
} from "./index.js"

type WasmErrorPayload = {
  readonly code?: string
  readonly operation?: string
  readonly details?: Record<string, unknown>
}

const U64_MAX = (1n << 64n) - 1n

function revisionArgument(value: unknown, operation: string, argument: string): bigint {
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

/** A persistable content snapshot containing a revision and Core Value. */
export class Snapshot {
  #handle: SnapshotHandle | undefined

  private constructor(handle: SnapshotHandle) {
    this.#handle = handle
  }

  /** Creates a Snapshot from a Core Value handle. */
  static fromValue(value: ValueHandle, revision: bigint = 0n): Snapshot {
    const operation = "snapshot_from_value"
    revision = revisionArgument(revision, operation, "revision")
    try {
      return new Snapshot(SnapshotHandle.fromValue(
        revision,
        ValueHandle._handle(value, operation),
      ))
    } catch (error) {
      if (error instanceof CollaError) throw error
      throw new CollaError("invalid_argument", operation, { reason: String(error) })
    }
  }

  /** Creates a Snapshot from structured Core Value input. */
  static fromJS(input: Parameters<typeof ValueHandle.fromJS>[0], revision: bigint = 0n): Snapshot {
    const value = ValueHandle.fromJS(input)
    try {
      return Snapshot.fromValue(value, revision)
    } finally {
      value.dispose()
    }
  }

  /** Decodes a local Snapshot envelope. */
  static decode(bytes: Uint8Array): Snapshot {
    if (!(bytes instanceof Uint8Array)) {
      throw new CollaError("invalid_argument", "snapshot_decode", {
        argument: "bytes",
        reason: "expected Uint8Array",
      })
    }
    try {
      return new Snapshot(SnapshotHandle.decode(bytes))
    } catch (error) {
      throw fromWasmError(error, "snapshot_decode")
    }
  }

  get revision(): bigint {
    return this.#get("snapshot_revision").revision()
  }

  /** Returns an independently owned ValueHandle for the content. */
  content(): ValueHandle {
    return ValueHandle._fromHandle(this.#get("snapshot_content").contentHandle())
  }

  encode(): Uint8Array {
    return new Uint8Array(this.#get("snapshot_encode").encode())
  }

  clone(): Snapshot {
    return new Snapshot(this.#get("snapshot_clone").cloneHandle())
  }

  dispose(): void {
    const handle = this.#handle
    if (handle === undefined) return
    this.#handle = undefined
    handle.free()
  }

  [Symbol.dispose](): void {
    this.dispose()
  }

  #get(operation: string): SnapshotHandle {
    if (this.#handle === undefined) {
      throw new CollaError("invalid_state", operation, {
        resource: "Snapshot",
        reason: "disposed",
      })
    }
    return this.#handle
  }
}

/** A versioned Core Change exchanged by a Document. */
export class Update {
  #handle: UpdateHandle | undefined

  private constructor(handle: UpdateHandle) {
    this.#handle = handle
  }

  /** Creates an Update around a Core Change. */
  static fromChange(revision: bigint, updateId: bigint, change: Change): Update {
    const operation = "update_from_change"
    revision = revisionArgument(revision, operation, "revision")
    updateId = revisionArgument(updateId, operation, "updateId")
    try {
      return new Update(UpdateHandle.fromChange(
        revision,
        updateId,
        Change._handle(change, operation),
      ))
    } catch (error) {
      if (error instanceof CollaError) throw error
      throw new CollaError("invalid_argument", operation, { reason: String(error) })
    }
  }

  /** Decodes a local Update envelope. */
  static decode(bytes: Uint8Array): Update {
    if (!(bytes instanceof Uint8Array)) {
      throw new CollaError("invalid_argument", "update_decode", {
        argument: "bytes",
        reason: "expected Uint8Array",
      })
    }
    try {
      return new Update(UpdateHandle.decode(bytes))
    } catch (error) {
      throw fromWasmError(error, "update_decode")
    }
  }

  get revision(): bigint {
    return this.#get("update_revision").revision()
  }

  get updateId(): bigint {
    return this.#get("update_id").updateId()
  }

  change(): Change {
    return Change._fromHandle(this.#get("update_change").changeHandle())
  }

  encode(): Uint8Array {
    return new Uint8Array(this.#get("update_encode").encode())
  }

  clone(): Update {
    return new Update(this.#get("update_clone").cloneHandle())
  }

  dispose(): void {
    const handle = this.#handle
    if (handle === undefined) return
    this.#handle = undefined
    handle.free()
  }

  [Symbol.dispose](): void {
    this.dispose()
  }

  #get(operation: string): UpdateHandle {
    if (this.#handle === undefined) {
      throw new CollaError("invalid_state", operation, {
        resource: "Update",
        reason: "disposed",
      })
    }
    return this.#handle
  }
}

/** A change event emitted when the visible Document content changes. */
export interface DocumentChangeEvent {
  readonly origin: "local" | "remote"
  readonly editSteps: readonly EditStep[]
  readonly revision: bigint
}

/** An error raised by a Document event listener. */
export interface DocumentErrorEvent {
  readonly error: unknown
}

type Pending = { update: Update; change: Change }
type ChangeListener = (event: DocumentChangeEvent) => unknown
type ErrorListener = (event: DocumentErrorEvent) => unknown

/** High-level mutable document state with local and remote update handling. */
export class Document {
  #value: ValueHandle
  #confirmedValue: ValueHandle
  #revision: bigint
  #confirmedRevision: bigint
  #nextUpdateId = 1n
  #pending: Pending[] = []
  #changeListeners = new Set<ChangeListener>()
  #errorListeners = new Set<ErrorListener>()
  #disposed = false

  private constructor(snapshot: Snapshot) {
    this.#value = snapshot.content()
    this.#confirmedValue = this.#value.clone()
    this.#revision = snapshot.revision
    this.#confirmedRevision = snapshot.revision
  }

  /** Restores a Document from a content Snapshot. */
  static fromSnapshot(snapshot: Snapshot): Document {
    if (!(snapshot instanceof Snapshot)) {
      throw new CollaError("invalid_argument", "document_from_snapshot", {
        argument: "snapshot",
        reason: "expected Snapshot",
      })
    }
    return new Document(snapshot)
  }

  /** Creates a Document from structured Core Value input. */
  static fromJS(input: Parameters<typeof ValueHandle.fromJS>[0], revision: bigint = 0n): Document {
    const snapshot = Snapshot.fromJS(input, revision)
    try {
      return Document.fromSnapshot(snapshot)
    } finally {
      snapshot.dispose()
    }
  }

  get revision(): bigint {
    this.#assertActive("document_revision")
    return this.#revision
  }

  /** Returns an independently owned handle for the visible content. */
  value(): ValueHandle {
    this.#assertActive("document_value")
    return this.#value.clone()
  }

  /** Encodes the visible content and current revision as a Snapshot. */
  snapshot(): Snapshot {
    this.#assertActive("document_snapshot")
    return Snapshot.fromValue(this.#value, this.#revision)
  }

  /** Applies a local Core Change optimistically and returns its Update. */
  applyLocal(change: Change): Update {
    this.#assertActive("document_apply_local")
    if (!(change instanceof Change)) {
      throw new CollaError("invalid_argument", "document_apply_local", {
        argument: "change",
        reason: "expected Change",
      })
    }
    if (this.#revision === U64_MAX) throw revisionOverflow("document_apply_local")
    if (this.#nextUpdateId > U64_MAX) throw updateIdOverflow("document_apply_local")

    const before = this.#value.clone()
    let next: ValueHandle
    try {
      next = apply(this.#value, change)
    } catch (error) {
      before.dispose()
      throw error
    }
    let update: Update
    try {
      update = Update.fromChange(this.#revision, this.#nextUpdateId, change)
    } catch (error) {
      next.dispose()
      before.dispose()
      throw error
    }
    this.#nextUpdateId += 1n
    this.#value.dispose()
    this.#value = next
    this.#revision += 1n
    this.#pending.push({ update: update.clone(), change: change.clone() })
    try {
      this.#emitChange("local", change, before)
    } finally {
      before.dispose()
    }
    return update
  }

  /** Applies a remote Update and rebases local pending changes. */
  applyRemote(update: Update): void {
    this.#assertActive("document_apply_remote")
    if (!(update instanceof Update)) {
      throw new CollaError("invalid_argument", "document_apply_remote", {
        argument: "update",
        reason: "expected Update",
      })
    }
    if (update.revision !== this.#confirmedRevision) {
      throw new CollaError("incompatible_change", "document_apply_remote", {
        reason: "revision_mismatch",
        expected: this.#confirmedRevision,
        actual: update.revision,
      })
    }
    if (this.#confirmedRevision === U64_MAX || this.#revision === U64_MAX) {
      throw revisionOverflow("document_apply_remote")
    }

    const before = this.#value.clone()
    let visibleChange = update.change()
    let confirmedNext: ValueHandle | undefined
    let visibleNext: ValueHandle | undefined
    const nextPending: Pending[] = []
    try {
      confirmedNext = apply(this.#confirmedValue, visibleChange)
      for (const pending of this.#pending) {
        const pendingId = pending.update.updateId
        const [pendingPrime, remotePrime] = transformPair(
          pending.change,
          visibleChange,
          { order: "left-first" },
        )
        visibleChange.dispose()
        visibleChange = remotePrime
        try {
          nextPending.push({
            update: Update.fromChange(
              this.#confirmedRevision + BigInt(nextPending.length + 1),
              pendingId,
              pendingPrime,
            ),
            change: pendingPrime,
          })
        } catch (error) {
          pendingPrime.dispose()
          throw error
        }
      }
      visibleNext = apply(this.#value, visibleChange)
    } catch (error) {
      visibleChange.dispose()
      before.dispose()
      confirmedNext?.dispose()
      visibleNext?.dispose()
      for (const pending of nextPending) {
        pending.update.dispose()
        pending.change.dispose()
      }
      throw error
    }

    for (const pending of this.#pending) {
      pending.update.dispose()
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

  /** Acknowledges the oldest pending local Update by its local updateId. */
  ack(updateId: bigint): void {
    this.#assertActive("document_ack")
    updateId = revisionArgument(updateId, "document_ack", "updateId")
    const pending = this.#pending[0]
    if (pending === undefined || pending.update.updateId !== updateId) {
      throw new CollaError("invalid_argument", "document_ack", {
        argument: "updateId",
        reason: "unknown_or_out_of_order_update",
      })
    }
    if (this.#confirmedRevision === U64_MAX) throw revisionOverflow("document_ack")
    const confirmedNext = apply(this.#confirmedValue, pending.change)
    this.#confirmedValue.dispose()
    this.#confirmedValue = confirmedNext
    this.#confirmedRevision += 1n
    pending.update.dispose()
    pending.change.dispose()
    this.#pending.shift()

    const nextPending: Pending[] = []
    for (const item of this.#pending) {
      const change = item.change
      const id = item.update.updateId
      item.update.dispose()
      nextPending.push({
        update: Update.fromChange(this.#confirmedRevision + BigInt(nextPending.length), id, change),
        change,
      })
    }
    this.#pending = nextPending
  }

  /** Subscribes to typed Document events and returns an unsubscribe function. */
  on(event: "change", listener: (event: DocumentChangeEvent) => unknown): () => void
  on(event: "error", listener: (event: DocumentErrorEvent) => unknown): () => void
  on(event: string, listener: unknown): () => void {
    this.#assertActive("document_on")
    if (typeof listener !== "function") {
      throw new CollaError("invalid_argument", "document_on", {
        argument: "listener",
        reason: "expected a function",
      })
    }
    if (event === "change") {
      const changeListener = listener as ChangeListener
      this.#changeListeners.add(changeListener)
      return () => this.#changeListeners.delete(changeListener)
    }
    if (event === "error") {
      const errorListener = listener as ErrorListener
      this.#errorListeners.add(errorListener)
      return () => this.#errorListeners.delete(errorListener)
    }
    throw new CollaError("invalid_argument", "document_on", {
      argument: "event",
      reason: "expected change or error",
    })
  }

  dispose(): void {
    if (this.#disposed) return
    this.#disposed = true
    this.#value.dispose()
    this.#confirmedValue.dispose()
    for (const pending of this.#pending) {
      pending.update.dispose()
      pending.change.dispose()
    }
    this.#pending = []
    this.#changeListeners.clear()
    this.#errorListeners.clear()
  }

  [Symbol.dispose](): void {
    this.dispose()
  }

  #emitChange(
    origin: DocumentChangeEvent["origin"],
    change: Change,
    before: ValueHandle,
  ): void {
    if (this.#changeListeners.size === 0) return
    const editSteps = convertChangeToEditSteps(change, before)
    const event: DocumentChangeEvent = Object.freeze({
      origin,
      editSteps,
      revision: this.#revision,
    })
    for (const listener of [...this.#changeListeners]) {
      try {
        listener(event)
      } catch (error) {
        this.#emitError(error)
      }
    }
  }

  #emitError(error: unknown): void {
    const event: DocumentErrorEvent = Object.freeze({ error })
    for (const listener of [...this.#errorListeners]) {
      try {
        listener(event)
      } catch {
        // Error listeners are terminal observers; never recurse on failures.
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
