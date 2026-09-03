import assert from "node:assert/strict"
import { test } from "node:test"

import { Document, Snapshot, Update } from "../dist/node.js"
import { Change, ValueHandle, text } from "../dist/core-node.js"

function testWithCleanup(name, callback) {
  test(name, () => {
    const cleanup = []
    const track = resource => {
      cleanup.push(resource)
      return resource
    }
    try {
      callback(track)
    } finally {
      for (const resource of cleanup.reverse()) resource.dispose()
    }
  })
}

testWithCleanup("Snapshot and Update local envelopes round-trip", track => {
  const value = track(ValueHandle.fromJS(text("Draft")))
  const snapshot = track(Snapshot.fromValue(value, 7n))
  const snapshotBytes = snapshot.encode()
  assert.deepEqual(snapshotBytes.slice(0, 6), Uint8Array.from([67, 79, 76, 76, 65, 83]))
  assert.equal(snapshot.revision, 7n)
  const decodedSnapshot = track(Snapshot.decode(snapshotBytes))
  assert.equal(decodedSnapshot.revision, 7n)
  assert.deepEqual(decodedSnapshot.content().toJS(), text("Draft"))

  const change = track(Change.build(edit => edit.text(textEdit => textEdit.retain(5).insert("!"))))
  const update = track(Update.fromChange(7n, 3n, change))
  const updateBytes = update.encode()
  assert.deepEqual(updateBytes.slice(0, 6), Uint8Array.from([67, 79, 76, 76, 65, 85]))
  const decodedUpdate = track(Update.decode(updateBytes))
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
  const maxSnapshot = track(Snapshot.fromJS(null, max))
  const noop = track(Change.build(edit => edit.noop()))
  const maxUpdate = track(Update.fromChange(max, max, noop))
  const decodedMaxSnapshot = track(Snapshot.decode(maxSnapshot.encode()))
  assert.equal(decodedMaxSnapshot.revision, max)
  const decodedMaxUpdate = track(Update.decode(maxUpdate.encode()))
  assert.equal(decodedMaxUpdate.revision, max)
  assert.equal(decodedMaxUpdate.updateId, max)
})

testWithCleanup("Document applies local and remote changes and emits edit steps", track => {
  const document = track(Document.fromJS(text("ab"), 0n))
  const events = []
  const unsubscribe = document.on("change", event => events.push(event))

  const localChange = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("X"))))
  const localUpdate = track(document.applyLocal(localChange))
  assert.equal(localUpdate.updateId, 1n)
  assert.equal(document.revision, 1n)
  assert.deepEqual(document.value().toJS(), text("aXb"))
  assert.equal(events.length, 1)
  assert.equal(events[0].origin, "local")
  assert.equal(events[0].revision, 1n)
  assert.equal(events[0].change, undefined)
  assert.equal(events[0].editSteps[0].type, "text")

  const remoteChange = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("Y"))))
  const remoteUpdate = track(Update.fromChange(0n, 99n, remoteChange))
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

testWithCleanup("Document isolates listener failures and reports them as error events", track => {
  const document = track(Document.fromJS(text("a")))
  const errors = []
  const changes = []
  document.on("error", event => errors.push(event.error))
  document.on("change", () => { throw new Error("first listener failed") })
  document.on("change", event => changes.push(event))

  const change = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b"))))
  const update = track(document.applyLocal(change))
  assert.equal(update.updateId, 1n)
  assert.equal(document.revision, 1n)
  assert.equal(changes.length, 1)
  assert.equal(errors.length, 1)
  assert.equal(errors[0].message, "first listener failed")

  document.on("error", () => { throw new Error("error listener failed") })
  const remoteChange = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("c"))))
  const remote = track(Update.fromChange(0n, 2n, remoteChange))
  document.applyRemote(remote)
  assert.equal(document.revision, 2n)
  assert.equal(changes.length, 2)
  assert.equal(errors.length, 2)
})

testWithCleanup("Document snapshot restores visible content and revision only", track => {
  const document = track(Document.fromJS(text("a"), 4n))
  const change = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b"))))
  const update = track(document.applyLocal(change))
  const snapshot = track(document.snapshot())
  const restored = track(Document.fromSnapshot(snapshot))
  assert.equal(snapshot.revision, 5n)
  assert.equal(restored.revision, 5n)
  assert.deepEqual(restored.value().toJS(), text("ab"))
  assert.throws(() => restored.ack(update.updateId), error =>
    error.code === "invalid_argument" && error.operation === "document_ack")
})

testWithCleanup("Document keeps revisions within unsigned 64-bit range", track => {
  const max = (1n << 64n) - 1n
  const document = track(Document.fromJS(text("a"), max - 1n))
  const change = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b"))))
  const update = track(document.applyLocal(change))
  assert.equal(document.revision, max)
  document.ack(update.updateId)
  assert.equal(document.revision, max)
  assert.throws(() => document.applyLocal(change), error =>
    error.code === "limit_exceeded" && error.operation === "document_apply_local")

  const atMax = track(Document.fromJS(text("a"), max))
  const remoteChange = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b"))))
  const remote = track(Update.fromChange(max, 10n, remoteChange))
  assert.throws(() => atMax.applyRemote(remote), error =>
    error.code === "limit_exceeded" && error.operation === "document_apply_remote")
})

testWithCleanup("Document accepts only the next server-ordered remote revision", track => {
  const document = track(Document.fromJS(text("a"), 4n))
  const change = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b"))))
  const skipped = track(Update.fromChange(5n, 1n, change))
  assert.throws(() => document.applyRemote(skipped), error =>
    error.code === "incompatible_change" &&
    error.operation === "document_apply_remote" &&
    error.details.expected === 4n && error.details.actual === 5n)
  assert.equal(document.revision, 4n)
  assert.deepEqual(document.value().toJS(), text("a"))
})

testWithCleanup("Document keeps state and pending changes intact when applying fails", track => {
  const document = track(Document.fromJS("a"))
  const invalid = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("!"))))
  assert.throws(() => document.applyLocal(invalid), error =>
    error.code === "type_mismatch" && error.operation === "apply")
  assert.equal(document.revision, 0n)
  assert.equal(document.value().toJS(), "a")

  const localChange = track(Change.build(edit => edit.replace("b")))
  const localUpdate = track(document.applyLocal(localChange))
  const invalidRemote = track(Update.fromChange(0n, 10n, invalid))
  assert.throws(() => document.applyRemote(invalidRemote), error =>
    error.code === "type_mismatch" && error.operation === "apply")
  assert.equal(document.revision, 1n)
  assert.equal(document.value().toJS(), "b")
  document.ack(localUpdate.updateId)
  assert.equal(document.revision, 1n)
})
