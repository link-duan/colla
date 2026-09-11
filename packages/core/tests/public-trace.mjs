// This exact program runs from the packed package in Node, browsers and workers.
export const tracer = `
import { Authority, Document, History, SessionCheckpoint, SyncSession, Value, ref, richText, text } from "colla-ot"
export function trace() {
  const doc = Document.create({ from: [{ title: text("A😀"), body: richText([]) }], to: [], selected: null })
  const id = doc.idAt(["from", 0])
  const history = History.attach(doc)
  const before = doc.snapshot()
  doc.edit(tx => {
    tx.text(["from", 0, "title"]).insert(3, "!")
    tx.richText(["from", 0, "body"]).insertEmbed(0, ref(id))
    tx.move(id, { parent: ["to"], index: 0 })
    tx.set(["selected"], ref(id))
  })
  if (doc.resolve(ref(id)).get(["title"]).toJS().value !== "A😀!") throw Error("Move/Ref failed")
  const after = doc.snapshot()
  history.undo()
  if (!before.equals(doc.snapshot())) throw Error("Undo identity failed")
  history.redo()
  if (!after.equals(Value.decode(doc.snapshot().encode()))) throw Error("codec identity failed")
  let authority = Authority.create({ documentId: "browser", value: after })
  let session = SyncSession.create({ clientId: "client", snapshot: authority.snapshot() })
  session.document.edit(tx => tx.set(["selected"], "after"))
  const checkpoint = session.checkpoint().encode()
  session.close()
  session = SyncSession.restore(SessionCheckpoint.decode(checkpoint))
  const accepted = authority.accept(session.outbound())
  authority = accepted.authority
  session.receive(accepted.message)
  if (session.outbound() !== null || !session.document.snapshot().equals(authority.snapshot().value)) throw Error("sync failed")
  const result = session.document.get(["selected"]).toJS()
  session.close(); doc.close()
  if (!Value.decode(after.encode()).equals(after)) throw Error("snapshot lifetime failed")
  return result
}
`
