import { Document } from 'colla-ot'

const doc = Document.create({
  title: 'Draft',
  temporary: true,
  steps: ['draft', 'publish'],
})
doc.edit(tx => {
  tx.set(['title'], 'Ready')
  tx.set(['approved'], true) // Add a final Map key.
  tx.delete(['temporary'])
  const steps = tx.list(['steps'])
  steps.insert(1, ['review'])
  console.log('After insertion:', tx.get(['steps'])?.toJS())
  steps.replace(2, 1, ['release']) // Index 2 now points to publish.
  steps.delete(0, 1)
})
console.log('Title:', doc.get(['title'])?.toJS()) // Ready
console.log('Approved:', doc.get(['approved'])?.toJS()) // true
console.log('Temporary field exists:', doc.has(['temporary'])) // false
console.log('Final steps:', doc.get(['steps'])?.toJS()) // ['review', 'release']
doc.close()
