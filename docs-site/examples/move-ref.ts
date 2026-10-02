import { Document, ref, text } from 'colla-ot'

const doc = Document.create({
  todo: [{ title: text('Draft') }],
  done: [],
  selected: null,
})
const task = doc.idAt(['todo', 0])
console.log('Path before move:', doc.pathOf(task))

doc.edit(tx => {
  tx.set(['selected'], ref(task))
  tx.move(task, { parent: ['done'], index: 0 })
})

console.log('Path after move:', doc.pathOf(task)) // ['done', 0]
console.log('Ref still targets the same task:', doc.resolve(ref(task))?.id === task) // true

doc.close()
