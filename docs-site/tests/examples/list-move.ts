import { Document, Text, Value, apply, text, transform } from 'colla-ot'

function title(value: Value | undefined): string | undefined {
  const projected = value?.toJS()
  return projected instanceof Text ? projected.value : undefined
}

const base = Value.fromJS({ tasks: [{ title: text('Draft') }, { title: text('Review') }] })
const left = Document.create(base)
const right = Document.create(base)
const movement = left.edit(tx => tx.list(['tasks']).move(0, 1))!.change
const edit = right.edit(tx => tx.text(['tasks', 0, 'title']).insert(5, '!'))!.change
const [movementAfter, editAfter] = transform(base, movement, edit, { priority: 'left' })
const merged = apply(apply(base, movement), editAfter)
if (!merged.equals(apply(apply(base, edit), movementAfter))) throw new Error('Transform diverged')
if (title(merged.get(['tasks', 1, 'title'])) !== 'Draft!') {
  throw new Error('Concurrent edit did not follow the moved element')
}
left.close()
right.close()
console.log('ListMove carries concurrent edits')
