import { Document } from 'colla-ot'

const doc = Document.create({ title: 'Draft', archive: [] })
doc.edit(tx => tx.copy([], ['archive', 0]))
console.log('Original title:', doc.get(['title'])?.toJS()) // Draft
console.log('Archived title:', doc.get(['archive', 0, 'title'])?.toJS()) // Draft
console.log('Archive inside copy:', doc.get(['archive', 0, 'archive'])?.toJS()) // []
doc.close()
