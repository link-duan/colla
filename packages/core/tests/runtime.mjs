import test from "node:test"
import assert from "node:assert/strict"
import "../dist/node.js"
import { Authority, AuthorityCheckpoint, Change, CollaError, Document, History, HistoryCheckpoint, Ref, ServerMessage, SessionCheckpoint, Submission, SyncSession, SyncSnapshot, Value, apply, compose, invert, ref, richText, text, transform } from "../dist/node.js"

const code = expected => error => error instanceof CollaError && error.code === expected
test("Text operations reject unknown and missing types before applying a change", () => {
  const base = Value.fromJS(text("abc"))
  for (const type of ["typo", "DELETE", "", undefined, null, 2]) {
    assert.throws(() => Change.create([
      { type: "text", target: base.id, operations: [{ type, length: 1 }] },
    ]), code("invalid_argument"))
  }
  const change = Change.create([{ type: "text", target: base.id, operations: [
    { type: "retain", length: 1 },
    { type: "delete", length: 1 },
    { type: "insert", text: "!" },
  ] }])
  const next = apply(base, change)
  assert.equal(next.toJS().value, "a!c")
  assert.equal(next.id, base.id)
  assert.equal(base.toJS().value, "abc")
})

function accept(authority, session) {
  const result = authority.accept(session.outbound())
  assert.equal(result.message.type, "commit", result.message.reason?.message)
  return result
}

test("complete editing, stable identities, Move steps, inverse and immutable snapshots", () => {
  const doc = Document.create({ blocks: [{ title: text("Draft"), body: richText([]) }], archived: [], featured: null, views: 0n })
  const history = History.attach(doc)
  assert.equal(History.attach(doc), history)
  const blockId = doc.idAt(["blocks", 0])
  const before = doc.snapshot()
  let escaped, escapedText, escapedRich, escapedList
  const edit = doc.edit(tx => {
    escaped = tx
    escapedText = tx.text(["blocks", 0, "title"])
    escapedRich = tx.richText(["blocks", 0, "body"])
    escapedList = tx.list(["archived"])
    escapedText.insert(5, " v2")
    escapedRich.insertText(0, "Hello")
    escapedRich.format(0, 5, { bold: true })
    escapedRich.insertEmbed(5, ref(blockId))
    tx.set(["featured"], ref(blockId))
    tx.move(blockId, { parent: ["archived"], index: 0 })
    tx.increment(["views"], 1n)
  })
  assert.deepEqual(doc.pathOf(blockId), ["archived", 0])
  assert.equal(doc.resolve(ref(blockId)).get(["title"]).toJS().value, "Draft v2")
  assert.equal(edit.editSteps.find(step => step.type === "move").target, blockId)
  assert.ok(apply(before, edit.change).equals(edit.after))
  assert.ok(apply(before, Change.create(edit.editSteps)).equals(edit.after))
  assert.ok(apply(edit.after, invert(before, edit.change)).equals(before))
  assert.ok(Value.decode(edit.after.encode()).equals(edit.after))
  for (const fn of [() => escaped.set(["views"], 9n), () => escaped.snapshot(), () => escapedText.delete(0, 1), () => escapedRich.insertText(0, "bad"), () => escapedList.insert(0, [null])]) assert.throws(fn, code("invalid_state"))
  assert.equal(doc.get(["views"]).toJS(), 1n)
  history.undo()
  assert.ok(doc.snapshot().equals(before))
  history.redo()
  assert.ok(doc.snapshot().equals(edit.after))
  doc.close(); history.close(); doc.close()
  assert.ok(edit.before.equals(before))
  assert.ok(Value.decode(edit.after.encode()).equals(edit.after))
  assert.equal("dispose" in edit.after, false)
  assert.equal("free" in edit.after, false)
})

test("UTF-16 coordinates use each working step and reject surrogate splits", () => {
  const doc = Document.create({ title: text("A😀B"), list: [0n] })
  const original = doc.snapshot()
  assert.throws(() => doc.edit(tx => tx.text(["title"]).insert(2, "bad")), code("invalid_utf16_boundary"))
  assert.ok(doc.snapshot().equals(original))
  doc.edit(tx => { const editor = tx.text(["title"]); editor.insert(3, "!"); editor.replace(1, 2, "🐬"); editor.delete(3, 1) })
  assert.equal(doc.get(["title"]).toJS().value, "A🐬B")
  assert.throws(() => doc.edit(tx => tx.text(["title"]).delete(99, 1)), code("out_of_bounds"))
  assert.throws(() => doc.edit(tx => tx.list(["list"]).replace(0, 2, [])), code("out_of_bounds"))
  assert.throws(() => doc.edit(tx => tx.delete([])), code("invalid_argument"))
  assert.throws(() => doc.edit(tx => tx.move([], { parent: ["list"], index: 0 })), code("invalid_argument"))
})

test("Noop, thenable and nested transactions cannot change version or pending", async () => {
  const authority = Authority.create({ documentId: "atomic", value: { count: 0n, title: text("") } })
  const session = SyncSession.create({ clientId: "a", snapshot: authority.snapshot() })
  const doc = session.document
  const history = History.attach(doc)
  let events = 0
  doc.subscribe(() => events++)
  const initial = session.checkpoint().encode()
  assert.equal(doc.edit(tx => tx.increment(["count"], 0n)), null)
  assert.throws(() => doc.edit(tx => { tx.increment(["count"], 1n); doc.edit(() => {}) }), code("invalid_state"))
  assert.throws(() => doc.edit(tx => { tx.increment(["count"], 1n); return { then() {} } }), code("invalid_state"))
  let escaped
  let thenableCalls = 0
  assert.throws(() => doc.edit(tx => { tx.increment(["count"], 1n); return { then() { thenableCalls++ } } }), code("invalid_state"))
  assert.throws(() => doc.edit(async tx => { escaped = tx; tx.increment(["count"], 1n); await Promise.resolve(); tx.increment(["count"], 1n) }), code("invalid_state"))
  await new Promise(resolve => setImmediate(resolve))
  assert.equal(thenableCalls, 0)
  assert.throws(() => escaped.increment(["count"], 1n), code("invalid_state"))
  assert.throws(() => doc.edit(tx => { tx.increment(["count"], 1n); doc.close() }), code("invalid_state"))
  assert.deepEqual(session.checkpoint().encode(), initial)
  assert.equal(doc.version, 0n); assert.equal(events, 0); assert.equal(history.canUndo, false); assert.equal(session.outbound(), null)
})

test("listener failures are isolated and both event channels reject reentry", () => {
  const authority = Authority.create({ documentId: "events", value: { count: 0n } })
  const session = SyncSession.create({ clientId: "a", snapshot: authority.snapshot() })
  const doc = session.document
  let diagnostics = 0, later = 0, syncEvents = 0
  doc.subscribe(() => { throw new Error("listener") }, { onError: () => diagnostics++ })
  doc.subscribe(event => {
    assert.ok(Object.isFrozen(event)); assert.equal(doc.get(["count"]).toJS(), 1n)
    assert.throws(() => doc.edit(tx => tx.increment(["count"], 1n)), code("invalid_state")); later++
  })
  const unsubscribe = session.subscribe(state => { assert.ok(Object.isFrozen(state)); assert.throws(() => doc.edit(() => {}), code("invalid_state")); syncEvents++ })
  doc.edit(tx => tx.increment(["count"], 1n))
  assert.equal(diagnostics, 1); assert.equal(later, 1); assert.equal(syncEvents, 1)
  const result = accept(authority, session); session.receive(result.message)
  assert.equal(later, 1); assert.equal(syncEvents, 2)
  unsubscribe(); unsubscribe()
  session.receive(result.message)
  assert.equal(syncEvents, 2)
})

test("Copy/set remap internal Ref targets and Undo restores dangling references", () => {
  const source = Document.create({ target: 1n, link: null })
  source.edit(tx => tx.set(["link"], ref(source.idAt(["target"]))))
  const doc = Document.create({ item: null, copied: null })
  doc.edit(tx => tx.set(["item"], source.snapshot()))
  const itemRoot = doc.idAt(["item"]), target = doc.idAt(["item", "target"])
  const link = doc.get(["item", "link"]).toJS()
  assert.ok(link instanceof Ref); assert.equal(link.target, target); assert.notEqual(target, source.idAt(["target"]))
  const snapshot = doc.snapshot()
  const history = History.attach(doc)
  doc.edit(tx => tx.delete(target))
  assert.equal(doc.resolve(link), undefined)
  assert.equal(snapshot.resolve(link).toJS(), 1n)
  history.undo()
  assert.equal(doc.resolve(link).toJS(), 1n)
  assert.equal(doc.idAt(["item"]), itemRoot)
  doc.edit(tx => { tx.delete(["copied"]); tx.copy(itemRoot, { parent: [], key: "copied" }) })
  const copied = doc.idAt(["copied", "target"])
  assert.equal(doc.get(["copied", "link"]).toJS().target, copied)
  assert.notEqual(copied, target)
})

test("controlled protocol objects, immutable retry bytes, gaps and full checkpoints", () => {
  let authority = Authority.create({ documentId: "sync", value: { count: 0n } })
  const initial = authority.snapshot()
  assert.throws(() => SyncSession.create({ clientId: "a", snapshot: { ...initial, revision: -1n } }), code("invalid_argument"))
  let a = SyncSession.create({ clientId: "a", snapshot: SyncSnapshot.decode(initial.encode()) })
  const b = SyncSession.create({ clientId: "b", snapshot: initial })
  History.attach(a.document)
  a.document.edit(tx => tx.increment(["count"], 1n))
  const original = a.outbound(), originalBytes = original.encode()
  const mutable = original.encode(); mutable.fill(0)
  a.document.edit(tx => tx.increment(["count"], 2n))
  assert.deepEqual(a.outbound().encode(), originalBytes)
  b.document.edit(tx => tx.increment(["count"], 10n))
  let result = accept(authority, b); authority = result.authority
  const remote = ServerMessage.decode(result.message.encode())
  const before = a.checkpoint().encode()
  assert.throws(() => a.document.edit(() => a.receive(remote)), code("invalid_state"))
  assert.deepEqual(a.checkpoint().encode(), before)
  a.receive(remote); b.receive(remote)
  assert.deepEqual(a.outbound().encode(), originalBytes)
  const checkpoint = a.checkpoint().encode()
  const oldDocument = a.document
  a.close()
  assert.throws(() => oldDocument.edit(tx => tx.increment(["count"], 1n)), code("invalid_state"))
  a = SyncSession.restore(SessionCheckpoint.decode(checkpoint))
  assert.deepEqual(a.outbound().encode(), originalBytes)
  result = accept(authority, a); authority = result.authority
  const duplicate = authority.accept(Submission.decode(originalBytes))
  assert.equal(duplicate.authority.revision, authority.revision)
  assert.deepEqual(duplicate.message.encode(), result.message.encode())
  a.receive(result.message); b.receive(result.message)
  assert.equal(a.document.get(["count"]).toJS(), 13n)
  assert.equal(a.outbound().sequence, 2n)
  result = accept(authority, a); authority = result.authority
  a.receive(result.message); b.receive(result.message)
  assert.equal(a.outbound(), null)
  assert.ok(a.document.snapshot().equals(authority.snapshot().value))
  authority = Authority.restore(AuthorityCheckpoint.decode(authority.checkpoint().encode()))
  assert.ok(authority.snapshot().value.equals(b.document.snapshot()))
  const late = SyncSession.create({ clientId: "late", snapshot: initial })
  const lateBefore = late.checkpoint().encode()
  assert.throws(() => late.receive(result.message), code("missing_revision"))
  assert.deepEqual(late.checkpoint().encode(), lateBefore)
  for (const message of authority.commitsSince(0n)) late.receive(message)
  assert.ok(late.document.snapshot().equals(authority.snapshot().value))
})

test("session checkpoints larger than 64 MiB restore with the original pending request", () => {
  const mib = 1024 * 1024
  const authority = Authority.create({ documentId: "large", value: {
    first: "x".repeat(12 * mib), second: "y".repeat(12 * mib), count: 0n,
  } })
  const session = SyncSession.create({ clientId: "a", snapshot: authority.snapshot() })
  let restored
  try {
    assert.ok(session.checkpoint().encode().length < 64 * mib)
    session.document.edit(tx => tx.increment(["count"], 1n))
    const pending = session.outbound().encode()
    const saved = session.checkpoint().encode()
    assert.ok(saved.length > 64 * mib)
    session.close()
    restored = SyncSession.restore(SessionCheckpoint.decode(saved))
    assert.deepEqual(restored.outbound().encode(), pending)
    assert.equal(restored.document.get(["count"]).toJS(), 1n)
    const result = accept(authority, restored)
    restored.receive(result.message)
    assert.equal(restored.outbound(), null)
    assert.ok(restored.document.snapshot().equals(result.authority.snapshot().value))
  } finally {
    restored?.close()
    session.close()
  }
})

test("grouped history checkpoints and public transform return order", () => {
  const doc = Document.create({ title: text("a") })
  const history = History.attach(doc)
  const before = doc.snapshot()
  doc.edit(tx => tx.text(["title"]).insert(1, "b"), { group: "typing" })
  doc.edit(tx => tx.text(["title"]).insert(2, "c"), { group: "typing" })
  const restored = Document.create(doc.snapshot())
  const restoredHistory = History.restore(restored, HistoryCheckpoint.decode(history.checkpoint().encode()))
  restoredHistory.undo(); assert.ok(restored.snapshot().equals(before)); assert.equal(restoredHistory.canUndo, false)
  restoredHistory.redo(); assert.ok(restored.snapshot().equals(doc.snapshot()))
  const leftDoc = Document.create(before), rightDoc = Document.create(before)
  const left = leftDoc.edit(tx => tx.text(["title"]).insert(1, "L")).change
  const right = rightDoc.edit(tx => tx.text(["title"]).insert(1, "R")).change
  const [leftAfterRight, rightAfterLeft] = transform(before, left, right, { priority: "left" })
  assert.ok(apply(apply(before, right), leftAfterRight).equals(apply(apply(before, left), rightAfterLeft)))
  assert.ok(apply(before, compose(before, left, rightAfterLeft)).equals(apply(apply(before, left), rightAfterLeft)))
})

test("version-2 codecs reject other types, old versions and trailing bytes", () => {
  const value = Value.fromJS({ n: 1n, title: text("😀") })
  const encoded = value.encode()
  assert.throws(() => Change.decode(encoded), code("invalid_encoding"))
  const old = encoded.slice(); old[5] = 1
  assert.throws(() => Value.decode(old), code("invalid_encoding"))
  const trailing = new Uint8Array(encoded.length + 1); trailing.set(encoded)
  assert.throws(() => Value.decode(trailing), code("invalid_encoding"))
  assert.throws(() => Value.fromJS(1n << 64n), code("integer_overflow"))
  assert.throws(() => Value.fromJS(Number.NaN), code("invalid_value"))
  assert.throws(() => Value.fromJS("\ud800"), code("invalid_value"))
  const cyclic = {}; cyclic.self = cyclic
  assert.throws(() => Value.fromJS(cyclic), code("invalid_value"))
  assert.ok(value.equals(Value.decode(encoded)))
})

test("one-hop cyclic Ref and canonical formatting floats", () => {
  const doc = Document.create({ a: null, b: null, body: richText([{ type: "text", text: "x", attrs: { scale: -0 } }]) })
  const a = doc.idAt(["a"]), b = doc.idAt(["b"])
  doc.edit(tx => { tx.set(a, ref(b)); tx.set(b, ref(a)) })
  assert.equal(doc.resolve(ref(a)).toJS().target, b)
  assert.equal(doc.resolve(ref(b)).toJS().target, a)
  assert.ok(doc.snapshot().equals(Value.decode(doc.snapshot().encode())))
  assert.equal(Object.is(doc.get(["body"]).toJS().spans[0].attrs.scale, -0), false)
  assert.throws(() => doc.edit(tx => tx.increment(a, 0n)), code("type_mismatch"))
})
