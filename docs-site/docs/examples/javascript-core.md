# JavaScript Core example

This example uses the low-level API to edit an immutable nested Value.
It does not create a Document or open a transport connection.

```ts
import { Change, ValueHandle, apply, text } from 'colla-ot/core'

const before = ValueHandle.fromJS({ title: text('Draft'), count: 1n })
const change = Change.build(edit => {
  edit.map(map => {
    map.modify('title', title => title.text(t => t.retain(5).insert(' v2')))
    map.modify('count', count => count.intAdd(1n))
  })
})
const after = apply(before, change)
console.log(after.toJS())
const encoded = change.encode()
const decoded = Change.decode(encoded)
const again = apply(before, decoded)
console.log(again.toJS())
before.dispose(); after.dispose(); change.dispose(); decoded.dispose(); again.dispose()
```

`text()` marks a collaborative sequence; an ordinary JavaScript string is atomic.
Text retain and delete lengths use Unicode scalar values, not UTF-16 code units.
An editor adapter converts UTF-16 positions against the current Value.
`Change.build()` is a synchronous builder over typed Core input.
The builder does not apply intermediate changes or own a Wasm handle.
`ValueHandle.fromJS()` returns an owned immutable Value.
`Change.fromJS()` accepts an explicit typed input shape.
`apply()` requires a compatible concrete base.
`compose()` combines sequential Changes.
`invert()` needs the original base for undo.
`transformPair()` handles concurrent same-base Changes.
Match `CollaError.code`, not message text.
Configure limits for untrusted structured JavaScript input.
Keep transport, authentication, and retries outside `colla-ot/core`.

## Reading the result

The `title` child is a Text value, so the insert is character-level.
The `count` child is an Int value, so the delta is checked.
The original `before` Value remains unchanged after `apply()`.
The returned `after` Value owns its allocation independently.
Encoding produces canonical Core Change bytes.
Decoding validates the complete byte sequence.
Trailing bytes are rejected by the decoder.
Malformed input raises a stable `CollaError`.
Edit Steps are projections for editors, not Change input.
Paths are relative to a particular base Value.

## Next

Read [OT algebra](../ot/algebra).
Read [editor integration](../document/editor-integration).
For optimistic mutable state, read [Document synchronization](./javascript-document).
