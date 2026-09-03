import assert from "node:assert/strict"
import { test } from "node:test"

import { Document, Snapshot, Update } from "../dist/node.js"
import { Change, ValueHandle, text } from "../dist/core-node.js"

test("Snapshot and Update local envelopes round-trip", () => {
  using value = ValueHandle.fromJS(text("Draft"))
  using snapshot = Snapshot.fromValue(value, 7n)
  const snapshotBytes = snapshot.encode()
  assert.deepEqual(snapshotBytes.slice(0, 6), Uint8Array.from([67, 79, 76, 76, 65, 83]))
  assert.equal(snapshot.revision, 7n)
  using decodedSnapshot = Snapshot.decode(snapshotBytes)
  assert.equal(decodedSnapshot.revision, 7n)
  assert.deepEqual(decodedSnapshot.content().toJS(), text("Draft"))

  using change = Change.build(edit => edit.text(textEdit => textEdit.retain(5).insert("!")))
  using update = Update.fromChange(7n, 3n, change)
  const updateBytes = update.encode()
  assert.deepEqual(updateBytes.slice(0, 6), Uint8Array.from([67, 79, 76, 76, 65, 85]))
  using decodedUpdate = Update.decode(updateBytes)
  assert.equal(decodedUpdate.revision, 7n)
  assert.equal(decodedUpdate.updateId, 3n)
  assert.deepEqual(decodedUpdate.change().encode(), change.encode())

  const invalidMagic = snapshotBytes.slice()
  invalidMagic[0] = 0
  assert.throws(() => Snapshot.decode(invalidMagic), error =>
    error.code === "invalid_encoding" && error.operation === "snapshot_decode" &&
    error.details.reason.includes("invalid snapshot envelope magic"))

  const invalidUpdateMagic = updateBytes.slice()
  invalidUpdateMagic[0] = 0
  assert.throws(() => Update.decode(invalidUpdateMagic), error =>
    error.code === "invalid_encoding" && error.operation === "update_decode" &&
    error.details.reason.includes("invalid update envelope magic"))

  const invalidUpdateVersion = updateBytes.slice()
  invalidUpdateVersion[6] = 2
  assert.throws(() => Update.decode(invalidUpdateVersion), error =>
    error.code === "invalid_encoding" && error.operation === "update_decode" &&
    error.details.reason.includes("unsupported update envelope protocol version 2"))

  const truncatedSnapshot = snapshotBytes.slice(0, 8)
  assert.throws(() => Snapshot.decode(truncatedSnapshot), error =>
    error.code === "invalid_encoding" && error.operation === "snapshot_decode")

  const trailingUpdate = new Uint8Array(updateBytes.length + 1)
  trailingUpdate.set(updateBytes)
  assert.throws(() => Update.decode(trailingUpdate), error =>
    error.code === "invalid_encoding" && error.operation === "update_decode" &&
    error.details.reason.includes("trailing bytes"))

  const max = (1n << 64n) - 1n
  using maxSnapshot = Snapshot.fromJS(null, max)
  using noop = Change.build(edit => edit.noop())
  using maxUpdate = Update.fromChange(max, max, noop)
  using decodedMaxSnapshot = Snapshot.decode(maxSnapshot.encode())
  assert.equal(decodedMaxSnapshot.revision, max)
  using decodedMaxUpdate = Update.decode(maxUpdate.encode())
  assert.equal(decodedMaxUpdate.revision, max)
  assert.equal(decodedMaxUpdate.updateId, max)
})

test("Document applies local and remote changes and emits edit steps", () => {
  using document = Document.fromJS(text("ab"), 0n)
  const events = []
  const unsubscribe = document.on("change", event => events.push(event))

  using localChange = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("X")))
  using localUpdate = document.applyLocal(localChange)
  assert.equal(localUpdate.updateId, 1n)
  assert.equal(document.revision, 1n)
  assert.deepEqual(document.value().toJS(), text("aXb"))
  assert.equal(events.length, 1)
  assert.equal(events[0].origin, "local")
  assert.equal(events[0].revision, 1n)
  assert.equal(events[0].change, undefined)
  assert.equal(events[0].editSteps[0].type, "text")

  using remoteChange = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("Y")))
  using remoteUpdate = Update.fromChange(0n, 99n, remoteChange)
  document.applyRemote(remoteUpdate)
  assert.equal(document.revision, 2n)
  assert.deepEqual(document.value().toJS(), text("aXYb"))
  assert.equal(events.length, 2)
  assert.equal(events[1].origin, "remote")
  assert.equal(events[1].revision, 2n)

  document.ack(localUpdate.updateId)
  assert.equal(events.length, 2)
  unsubscribe()
})

test("Document isolates listener failures and reports them as error events", () => {
  using document = Document.fromJS(text("a"))
  const errors = []
  const changes = []
  document.on("error", event => errors.push(event.error))
  document.on("change", () => { throw new Error("first listener failed") })
  document.on("change", event => changes.push(event))

  using change = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b")))
  using update = document.applyLocal(change)
  assert.equal(update.updateId, 1n)
  assert.equal(document.revision, 1n)
  assert.equal(changes.length, 1)
  assert.equal(errors.length, 1)
  assert.equal(errors[0].message, "first listener failed")

  document.on("error", () => { throw new Error("error listener failed") })
  using remoteChange = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("c")))
  using remote = Update.fromChange(0n, 2n, remoteChange)
  document.applyRemote(remote)
  assert.equal(document.revision, 2n)
  assert.equal(changes.length, 2)
  assert.equal(errors.length, 2)
})

test("Document snapshot restores visible content and revision only", () => {
  using document = Document.fromJS(text("a"), 4n)
  using change = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b")))
  using update = document.applyLocal(change)
  using snapshot = document.snapshot()
  using restored = Document.fromSnapshot(snapshot)
  assert.equal(snapshot.revision, 5n)
  assert.equal(restored.revision, 5n)
  assert.deepEqual(restored.value().toJS(), text("ab"))
  assert.throws(() => restored.ack(update.updateId), error =>
    error.code === "invalid_argument" && error.operation === "document_ack")
})

test("Document keeps revisions within unsigned 64-bit range", () => {
  const max = (1n << 64n) - 1n
  using document = Document.fromJS(text("a"), max - 1n)
  using change = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b")))
  using update = document.applyLocal(change)
  assert.equal(document.revision, max)
  document.ack(update.updateId)
  assert.equal(document.revision, max)
  assert.throws(() => document.applyLocal(change), error =>
    error.code === "limit_exceeded" && error.operation === "document_apply_local")

  using atMax = Document.fromJS(text("a"), max)
  using remoteChange = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b")))
  using remote = Update.fromChange(max, 10n, remoteChange)
  assert.throws(() => atMax.applyRemote(remote), error =>
    error.code === "limit_exceeded" && error.operation === "document_apply_remote")
})

test("Document accepts only the next server-ordered remote revision", () => {
  using document = Document.fromJS(text("a"), 4n)
  using change = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b")))
  using skipped = Update.fromChange(5n, 1n, change)
  assert.throws(() => document.applyRemote(skipped), error =>
    error.code === "incompatible_change" &&
    error.operation === "document_apply_remote" &&
    error.details.expected === 4n && error.details.actual === 5n)
  assert.equal(document.revision, 4n)
  assert.deepEqual(document.value().toJS(), text("a"))
})

test("Document keeps state and pending changes intact when applying fails", () => {
  using document = Document.fromJS("a")
  using invalid = Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("!")))
  assert.throws(() => document.applyLocal(invalid), error =>
    error.code === "type_mismatch" && error.operation === "apply")
  assert.equal(document.revision, 0n)
  assert.equal(document.value().toJS(), "a")

  using localChange = Change.build(edit => edit.replace("b"))
  using localUpdate = document.applyLocal(localChange)
  using invalidRemote = Update.fromChange(0n, 10n, invalid)
  assert.throws(() => document.applyRemote(invalidRemote), error =>
    error.code === "type_mismatch" && error.operation === "apply")
  assert.equal(document.revision, 1n)
  assert.equal(document.value().toJS(), "b")
  document.ack(localUpdate.updateId)
  assert.equal(document.revision, 1n)
})
