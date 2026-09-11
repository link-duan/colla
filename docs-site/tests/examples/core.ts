import { Change, Value, apply, compose, invert, text, transform } from 'colla-ot'

const base = Value.fromJS(text('A😀B'))
const left = Change.create([{
  type: 'text', target: base.id,
  operations: [{ type: 'retain', length: 2 }, { type: 'insert', text: '!' }],
}])
const right = Change.create([{
  type: 'text', target: base.id,
  operations: [{ type: 'insert', text: 'Hello ' }],
}])
const [leftAfterRight, rightAfterLeft] = transform(base, left, right, { priority: 'left' })
const merged = apply(apply(base, left), rightAfterLeft)
if (!merged.equals(apply(apply(base, right), leftAfterRight))) throw new Error('TP1 failed')
if (!merged.equals(apply(base, compose(base, left, rightAfterLeft)))) throw new Error('Compose failed')
if (!apply(apply(base, left), invert(base, left)).equals(base)) throw new Error('Inverse failed')
if (!Value.decode(merged.encode()).equals(merged)) throw new Error('Codec lost identity')
console.log(merged.toJS()) // Text { type: 'text', value: 'Hello A😀!B' }
