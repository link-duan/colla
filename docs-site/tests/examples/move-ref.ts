import { Document, History, Ref, ref, text } from 'colla-ot'

const doc = Document.create({ todo: [{ title: text('Draft'), link: null }], done: [], selected: null })
const history = History.attach(doc)
const item = doc.idAt(['todo', 0])
const title = doc.idAt(['todo', 0, 'title'])
doc.edit(tx => {
  tx.set(['todo', 0, 'link'], ref(title))
  tx.set(['selected'], ref(item))
})
doc.edit(tx => tx.move(item, { parent: ['done'], index: 0 }))
if (doc.idAt(['done', 0, 'title']) !== title) throw new Error('Move changed descendant ID')
if (doc.resolve(ref(item))?.id !== item) throw new Error('Reference did not follow move')
doc.edit(tx => tx.copy(item, { parent: ['todo'], index: 0 }))
const copiedTitle = doc.idAt(['todo', 0, 'title'])
const copiedRef = doc.get(['todo', 0, 'link'])?.toJS()
if (copiedTitle === title || !(copiedRef instanceof Ref) || copiedRef.target !== copiedTitle) {
  throw new Error('Copy failed to remap its internal Ref')
}
doc.edit(tx => tx.delete(item))
if (doc.resolve(ref(item)) !== undefined) throw new Error('Deleted target still resolves')
history.undo()
if (doc.resolve(ref(item))?.id !== item) throw new Error('Undo lost target identity')
history.close()
doc.close()
console.log('Move preserves IDs; Copy remaps internal Refs; Undo restores targets')
