import { Document, text } from 'colla-ot'

const doc = Document.create({ title: text('Draft') })
console.log('Before edit:', doc.get(['title'])?.toJS())

doc.edit(tx => {
  tx.text(['title']).insert(5, ' updated')
})

console.log('After edit:', doc.get(['title'])?.toJS()) // Text containing 'Draft updated'
doc.close()
