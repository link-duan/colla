import { Change, Value, apply, text, transform } from 'colla-ot'

const base = Value.fromJS(text('Hello'))
const left = Change.create([{
  type: 'text',
  target: base.id,
  operations: [{ type: 'insert', text: 'Say ' }],
}])
const right = Change.create([{
  type: 'text',
  target: base.id,
  operations: [{ type: 'retain', length: 5 }, { type: 'insert', text: '!' }],
}])

console.log('Left edit alone:', apply(base, left).toJS())
console.log('Right edit alone:', apply(base, right).toJS())

// Rebase the right edit so its position accounts for the inserted prefix.
const [, rightAfterLeft] = transform(base, left, right, { priority: 'left' })
const result = apply(apply(base, left), rightAfterLeft)

console.log('Merged edits:', result.toJS()) // Text containing 'Say Hello!'
