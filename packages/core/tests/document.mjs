import assert from "node:assert/strict"
import { test } from "node:test"

import { Document, Snapshot, Update, _createUpdate } from "../dist/node.js"
import { apply, Change, ValueHandle, text } from "../dist/node.js"

function testWithCleanup(name, callback) {
  test(name, () => {
    const cleanup = []
    const track = resource => {
      if (resource && typeof resource.dispose === "function") {
        cleanup.push(resource)
      }
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
  const snapshot = Snapshot.fromValue(value, 7n)
  const snapshotBytes = snapshot.bytes
  assert.deepEqual(snapshotBytes.slice(0, 6), Uint8Array.from([67, 79, 76, 76, 65, 83]))
  assert.equal(snapshot.revision, 7n)
  const decodedSnapshot = Snapshot.decode(snapshotBytes)
  assert.equal(decodedSnapshot.revision, 7n)
  assert.deepEqual(decodedSnapshot.value, text("Draft"))

  const doc = track(Document.fromJS(text("Draft"), 7n))
  const update = doc.transact(tx => tx.text([], t => t.retain(5).insert("!")))
  const updateBytes = update.bytes
  assert.deepEqual(updateBytes.slice(0, 6), Uint8Array.from([67, 79, 76, 76, 65, 85]))
  const decodedUpdate = Update.decode(updateBytes)
  assert.equal(decodedUpdate.revision, 7n)
  assert.equal(decodedUpdate.updateId, 1n)
  assert.deepEqual(decodedUpdate.bytes, updateBytes)

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
  const maxSnapshot = Snapshot.fromJS(null, max)
  const decodedMaxSnapshot = Snapshot.decode(maxSnapshot.bytes)
  assert.equal(decodedMaxSnapshot.revision, max)

  const noop = track(Change.build(edit => edit.noop()))
  const maxUpdate = _createUpdate(max, max, noop)
  const decodedMaxUpdate = Update.decode(maxUpdate.bytes)
  assert.equal(decodedMaxUpdate.revision, max)
  assert.equal(decodedMaxUpdate.updateId, max)
})

testWithCleanup("Document applies local and remote changes and emits edit steps", track => {
  const document = track(Document.fromJS(text("ab"), 0n))
  const events = []
  const unsubscribe = document.subscribe(event => events.push(event))

  const localUpdate = document.transact(tx => tx.text([], t => t.retain(1).insert("X")))
  assert.equal(localUpdate.updateId, 1n)
  assert.equal(document.revision, 1n)
  assert.deepEqual(document.value().toJS(), text("aXb"))
  assert.equal(events.length, 1)
  assert.equal(events[0].origin, "local")
  assert.equal(events[0].revision, 1n)
  assert.equal(events[0].editSteps[0].type, "text")

  const peerDoc = track(Document.fromJS(text("ab"), 0n))
  const remoteUpdate = peerDoc.transact(tx => tx.text([], t => t.retain(1).insert("Y")))
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

testWithCleanup("Document isolates listener failures and reports them via onError subscriber", track => {
  const document = track(Document.fromJS(text("a")))
  const errors = []
  const changes = []
  document.subscribe({
    onChange: event => {
      changes.push(event)
      throw new Error("first listener failed")
    },
    onError: err => errors.push(err),
  })
  document.subscribe(event => changes.push(event))

  const update = document.transact(tx => tx.text([], t => t.retain(1).insert("b")))
  assert.equal(update.updateId, 1n)
  assert.equal(document.revision, 1n)
  assert.equal(changes.length, 2)
  assert.equal(errors.length, 1)
  assert.equal(errors[0].message, "first listener failed")

  const peerDoc = track(Document.fromJS(text("a"), 0n))
  const remote = peerDoc.transact(tx => tx.text([], t => t.retain(1).insert("c")))
  document.applyRemote(remote.bytes)
  assert.equal(document.revision, 2n)
  assert.equal(changes.length, 4)
  assert.equal(errors.length, 2)
})

testWithCleanup("Document snapshot restores visible content and revision only", track => {
  const document = track(Document.fromJS(text("a"), 4n))
  const update = document.transact(tx => tx.text([], t => t.retain(1).insert("b")))
  const snapshot = document.snapshot()
  const restored = track(Document.fromSnapshot(snapshot))
  assert.equal(snapshot.revision, 5n)
  assert.equal(restored.revision, 5n)
  assert.deepEqual(restored.value().toJS(), text("ab"))
  assert.throws(() => restored.ack(update.updateId), error =>
    error.code === "invalid_argument" && error.operation === "document_ack")

  const restoredFromBytes = track(Document.fromSnapshot(snapshot.bytes))
  assert.equal(restoredFromBytes.revision, 5n)
  assert.deepEqual(restoredFromBytes.value().toJS(), text("ab"))
})

testWithCleanup("Document keeps revisions within unsigned 64-bit range", track => {
  const max = (1n << 64n) - 1n
  const document = track(Document.fromJS(text("a"), max - 1n))
  const update = document.transact(tx => tx.text([], t => t.retain(1).insert("b")))
  assert.equal(document.revision, max)
  document.ack(update.updateId)
  assert.equal(document.revision, max)
  assert.throws(() => document.transact(tx => tx.text([], t => t.retain(1).insert("b"))), error =>
    error.code === "limit_exceeded" && error.operation === "document_transact")

  const atMax = track(Document.fromJS(text("a"), max))
  const remoteChange = track(Change.build(edit => edit.text(textEdit => textEdit.retain(1).insert("b"))))
  const remote = _createUpdate(max, 10n, remoteChange)
  assert.throws(() => atMax.applyRemote(remote), error =>
    error.code === "limit_exceeded" && error.operation === "document_apply_remote")
})

testWithCleanup("Document accepts only the next server-ordered remote revision", track => {
  const document = track(Document.fromJS(text("a"), 4n))
  const peer = track(Document.fromJS(text("a"), 5n))
  const skipped = peer.transact(tx => tx.text([], t => t.retain(1).insert("b")))
  assert.throws(() => document.applyRemote(skipped), error =>
    error.code === "incompatible_change" &&
    error.operation === "document_apply_remote" &&
    error.details.expected === 4n && error.details.actual === 5n)
  assert.equal(document.revision, 4n)
  assert.deepEqual(document.value().toJS(), text("a"))
})

testWithCleanup("Document keeps state and pending changes intact when applying fails", track => {
  const document = track(Document.fromJS("a"))
  assert.throws(() => document.transact(tx => tx.text([], t => t.retain(1).insert("!"))), error =>
    error.code === "type_mismatch" && error.operation === "apply")
  assert.equal(document.revision, 0n)
  assert.equal(document.value().toJS(), "a")

  const localUpdate = document.transact(tx => tx.set([], "b"))
  const peer = track(Document.fromJS(text("a"), 0n))
  const invalidRemote = peer.transact(tx => tx.text([], t => t.retain(1).insert("!")))
  assert.throws(() => document.applyRemote(invalidRemote), error =>
    error.code === "type_mismatch" && error.operation === "apply")
  assert.equal(document.revision, 1n)
  assert.equal(document.value().toJS(), "b")
  document.ack(localUpdate.updateId)
  assert.equal(document.revision, 1n)
})

testWithCleanup("Change introspects kind and isNoop without base context", track => {
  const noop = track(Change.build(edit => edit.noop()))
  assert.equal(noop.kind(), "noop")
  assert.equal(noop.isNoop(), true)

  const textChange = track(Change.build(edit => edit.text(t => t.insert("hi"))))
  assert.equal(textChange.kind(), "text")
  assert.equal(textChange.isNoop(), false)

  const replaceChange = track(Change.build(edit => edit.replace("v")))
  assert.equal(replaceChange.kind(), "replace")
  assert.equal(replaceChange.isNoop(), false)
})

testWithCleanup("Document provides direct query methods without leaking handles", track => {
  const document = track(Document.fromJS({ title: text("Colla"), count: 42n, items: ["a", "b"] }, 0))
  assert.equal(document.revision, 0n)
  assert.equal(document.kind(), "map")
  assert.equal(document.kind(["title"]), "text")
  assert.equal(document.kind(["count"]), "int")
  assert.equal(document.kind(["items"]), "list")
  assert.equal(document.has(["title"]), true)
  assert.equal(document.has(["missing"]), false)
  assert.deepEqual(document.get(["title"]), text("Colla"))
  assert.equal(document.get(["count"]), 42n)
  assert.deepEqual(document.get(["items", 1]), "b")

  const update = document.transact(tx => {
    tx.text(["title"], s => s.retain(5).insert(" OT"))
  })
  assert.equal(update.updateId, 1n)
  assert.equal(document.revision, 1n)
  assert.deepEqual(document.get(["title"]), text("Colla OT"))

  document.ack(1)
  assert.equal(document.revision, 1n)

  assert.equal(document.resolveCodePointPosition(["title"], 5), 5)
  assert.equal(document.resolveUtf16Position(["title"], 5), 5)
})

testWithCleanup("Document supports cumulative ack and state queries", track => {
  const doc = track(Document.fromJS({ counter: 0 }))
  assert.equal(doc.hasPending, false)
  assert.equal(doc.pendingCount, 0)
  assert.equal(doc.confirmedRevision, 0n)

  doc.transact(tx => tx.set(["counter"], 1))
  doc.transact(tx => tx.set(["counter"], 2))
  const last = doc.transact(tx => tx.set(["counter"], 3))

  assert.equal(doc.hasPending, true)
  assert.equal(doc.pendingCount, 3)
  assert.equal(doc.revision, 3n)
  assert.equal(doc.confirmedRevision, 0n)

  // Cumulative ack up to updateId 3
  doc.ack(last.updateId)
  assert.equal(doc.hasPending, false)
  assert.equal(doc.pendingCount, 0)
  assert.equal(doc.confirmedRevision, 3n)
})

testWithCleanup("Document executes multi-mutation atomic transactions", track => {
  const doc = track(Document.fromJS({ a: "first", b: "second", list: [1, 2] }))
  const update = doc.transact(tx => {
    tx.set(["a"], "A")
    tx.set(["b"], "B")
    tx.list(["list"], l => l.retain(1).insert([99]))
  })

  assert.deepEqual(doc.get(["a"]), "A")
  assert.deepEqual(doc.get(["b"]), "B")
  assert.deepEqual(doc.get(["list"]), [1, 99, 2])

  // Peer receives binary bytes and reproduces identical state
  const peer = track(Document.fromJS({ a: "first", b: "second", list: [1, 2] }))
  peer.applyRemote(update.bytes)
  assert.deepEqual(peer.get(["a"]), "A")
  assert.deepEqual(peer.get(["b"]), "B")
  assert.deepEqual(peer.get(["list"]), [1, 99, 2])
})

testWithCleanup("CollaError message includes reason and path context", () => {
  const base = ValueHandle.fromJS("hello")
  const invalid = Change.build(edit => edit.text(t => t.retain(1).insert("!")))
  try {
    apply(base, invalid)
    assert.fail("should have thrown")
  } catch (error) {
    assert.ok(error.is("type_mismatch"))
    assert.ok(error.message.includes("apply failed: type_mismatch"))
  } finally {
    base.dispose()
    invalid.dispose()
  }
})


