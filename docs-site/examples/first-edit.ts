import { Document, text } from 'colla-ot'

const doc = Document.create({ title: text('Draft') })
console.log('Before edit:', doc.get(['title'])?.toJS())

doc.edit(tx => {
  tx.text(['title']).insert(5, ' v2')
})

console.log('After edit:', doc.get(['title'])?.toJS()) // Text containing 'Draft v2'
doc.close()
