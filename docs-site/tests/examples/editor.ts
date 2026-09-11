import { Change, Document, Text, Value, apply, text } from 'colla-ot'

const doc = Document.create({ title: text('A😀B') })
let rendered: Value = doc.snapshot()
const unsubscribe = doc.subscribe(event => {
  // Keep a model mirror using scalar Edit Steps, without feeding UI events back.
  rendered = apply(rendered, Change.create(event.editSteps))
  if (!rendered.equals(event.after)) throw new Error('Editor mirror diverged')
}, { onError: error => { console.error(error) } })
function replaceSelection(start: number, end: number, inserted: string): void {
  // Browser selection offsets are UTF-16; high-level editing accepts them directly.
  doc.edit(tx => tx.text(['title']).replace(start, end - start, inserted))
}
replaceSelection(1, 3, '🐬')
const title = rendered.get(['title'])?.toJS()
if (!(title instanceof Text) || title.value !== 'A🐬B') throw new Error('Unexpected rendered text')
if (!rendered.equals(doc.snapshot())) throw new Error('Missed event')
unsubscribe()
doc.close()
console.log('UTF-16 editor input and scalar model projection agree')
