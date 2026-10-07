import { Change, Value, apply, compose, invert, text, transform } from 'colla-ot'

const base = Value.fromJS(text('Hello'))
const left = Change.create([
  {
    type: 'text',
    path: [],
    operations: [{ type: 'insert', text: 'Say ' }],
  },
])
const right = Change.create([
  {
    type: 'text',
    path: [],
    operations: [
      { type: 'retain', length: 5 },
      { type: 'insert', text: '!' },
    ],
  },
])

console.log('Left edit alone:', apply(base, left).toJS())
console.log('Right edit alone:', apply(base, right).toJS())

// Rebase the right edit so its position accounts for the inserted prefix.
const [, rightAfterLeft] = transform(base, left, right, { priority: 'left' })
const result = apply(apply(base, left), rightAfterLeft)
console.log('Merged edits:', result.toJS()) // Text containing 'Say Hello!'

// Combine both edits into one Change, then build its inverse.
const combined = compose(base, left, rightAfterLeft)
console.log('Composed:', apply(base, combined).toJS()) // Text containing 'Say Hello!'
console.log('Inverted:', apply(result, invert(base, combined)).toJS()) // Text containing 'Hello'
