# Grouping and capacity

By default each successful local transaction is one undo unit. History retains 100
units unless capacity is supplied when first attaching it. Capacity bounds retained
undo intent; it is not an Authority log retention setting.

## Explicit grouping

```ts
import { Document, History, text } from 'colla-ot'
const doc = Document.create({ title: text('') })
const history = History.attach(doc, { capacity: 50 })
doc.edit(tx => tx.text(['title']).insert(0, 'H'), { group: 'typing-1' })
doc.edit(tx => tx.text(['title']).insert(1, 'i'), { group: 'typing-1' })
history.undo() // removes both insertions
console.log('More undo available:', history.canUndo) // false
history.close()
doc.close()
```

Only consecutive equal explicit group names merge. Generate a new group for the next
user gesture. Remote commits and undo/redo end a group; reusing its string afterward
does not bridge that boundary. The library does not group by elapsed time.

## Noops and new edits

A Noop local transaction creates no history unit and does not advance version. New
local work clears redo. Grouping does not weaken transaction atomicity: each transaction
still commits independently and can already be visible remotely before the group ends.

## Product implications

Choose capacity according to expected editing sessions and memory constraints. Calling
attach again returns the existing History; it is not a capacity reconfiguration API.
Persist a checkpoint when undo must survive restart. See [Checkpoints](./checkpoints).
