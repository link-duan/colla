// This exact program runs from the packed package in Node, browsers and workers.
export const tracer = `
import { Authority, Document, History, SessionCheckpoint, SyncSession, Value, richText, text } from "colla-ot"
export function trace() {
  const doc = Document.create({ items: [{ title: text("A😀"), body: richText([]) }, { title: text("B") }], selected: null })
  const history = History.attach(doc)
  const before = doc.snapshot()
  doc.edit(tx => {
    tx.text(["items", 0, "title"]).insert(3, "!")
    tx.richText(["items", 0, "body"]).insertEmbed(0, 1n)
    tx.list(["items"]).move(0, 1)
    tx.set(["chosen"], tx.get(["items", 1, "title"]))
  })
  if (doc.get(["items", 1, "title"]).toJS().value !== "A😀!") throw Error("ListMove failed")
  const after = doc.snapshot()
  history.undo()
  if (!before.equals(doc.snapshot())) throw Error("Undo failed")
  history.redo()
  if (!after.equals(Value.decode(doc.snapshot().encode()))) throw Error("codec failed")
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
