import { CollaError, Document, History, Text, Value, text } from 'colla-ot'

const doc = Document.create({ title: text('A😀B'), views: 0n })
const history = History.attach(doc)
const before = doc.snapshot()
doc.edit(tx => {
  tx.text(['title']).insert(3, '!') // UTF-16: after the emoji
  tx.increment(['views'], 1n)
})
const title = doc.get(['title'])?.toJS()
if (!(title instanceof Text) || title.value !== 'A😀!B') throw new Error('Unexpected title')
const edited = doc.snapshot()
try {
  doc.edit(tx => {
    tx.increment(['views'], 1n)
    tx.text(['title']).insert(2, 'invalid') // splits the surrogate pair
  })
  throw new Error('Expected a boundary error')
} catch (error) {
  if (!(error instanceof CollaError) || error.code !== 'invalid_utf16_boundary') throw error
}
if (!doc.snapshot().equals(edited)) throw new Error('Transaction did not roll back')
history.undo()
if (!doc.snapshot().equals(before)) throw new Error('Undo did not restore identity')
const restored = Document.create(Value.decode(doc.snapshot().encode()))
if (!restored.snapshot().equals(before)) throw new Error('Snapshot restore failed')
history.close()
doc.close()
restored.close()
console.log('First edit, atomic rollback and restoration passed')
