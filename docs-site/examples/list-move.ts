import { Document, text } from 'colla-ot'

const doc = Document.create({ tasks: [{ title: text('Draft') }, { title: text('Review') }] })
doc.edit(tx => tx.list(['tasks']).move(0, 1)) // The destination counts after removal.
console.log('First task:', doc.get(['tasks', 0, 'title'])?.toJS()) // Text containing 'Review'
console.log('Second task:', doc.get(['tasks', 1, 'title'])?.toJS()) // Text containing 'Draft'
doc.close()
