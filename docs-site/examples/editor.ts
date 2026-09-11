import { Change, Document, apply, text } from 'colla-ot'

const doc = Document.create({ title: text('A😀B') })
let rendered = doc.snapshot()

const unsubscribe = doc.subscribe(event => {
  const textStep = event.editSteps.find(step => step.type === 'text')
  console.log('Edit operations in Unicode scalars:', JSON.stringify(textStep?.operations))
  rendered = apply(rendered, Change.create(event.editSteps))
})

function replaceSelection(start: number, end: number, inserted: string) {
  doc.edit(tx => tx.text(['title']).replace(start, end - start, inserted))
}

console.log('Selected UTF-16 range: [1, 3) — one emoji, two code units')
replaceSelection(1, 3, '🐬') // Browser selection offsets use UTF-16.
console.log('Rendered text:', rendered.get(['title'])?.toJS()) // Text containing 'A🐬B'

unsubscribe()
doc.close()
