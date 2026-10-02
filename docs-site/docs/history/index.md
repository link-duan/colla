# Undo and redo

History tracks local editing intent and rebases it across remote changes. Attach it to
a standalone Document or the Document owned by a SyncSession.

## Attach and use

```ts
import { Document, History } from 'colla-ot'
const doc = Document.create({ count: 0n })
const history = History.attach(doc)
doc.edit(tx => tx.increment(['count'], 1n))
if (history.canUndo) history.undo()
console.log('After undo:', doc.get(['count'])?.toJS()) // 0n
if (history.canRedo) history.redo()
console.log('After redo:', doc.get(['count'])?.toJS()) // 1n
history.close()
doc.close()
```

`History.attach` returns the existing attached instance when present. Undo and redo
return an EditResult or null when no effective unit remains. Their origins are undo
and redo, and they are ordinary outgoing local work in synchronized sessions.

## What an undo restores

Inverses restore element identities, not merely equal-looking content. A Ref whose
target is restored can resolve again. History is not a saved sequence of full server
snapshots and does not rewind the Authority's revision.

## Boundaries

A new local edit clears redo; remote editing rebases it. `clear()` empties the history
stacks. Closing History ends tracking. Use the normal application permission checks for
undo/redo, since they can delete or restore content just like other local edits.

Continue with [Grouping](./grouping), [Remote changes](./rebasing), and
[Checkpoints](./checkpoints) before implementing collaborative undo controls.
