import { Document } from 'colla-ot'

const doc = Document.create({ title: 'Draft', archive: [] })
const original = doc.idAt([])
doc.edit(tx => {
  const copied = tx.copy([], { parent: ['archive'], index: 0 })
  console.log('Copy has a new ID:', copied !== original) // true
})
console.log('Original title:', doc.get(['title'])?.toJS()) // Draft
console.log('Archived title:', doc.get(['archive', 0, 'title'])?.toJS()) // Draft
console.log('Archive inside copy:', doc.get(['archive', 0, 'archive'])?.toJS()) // []
doc.close()
