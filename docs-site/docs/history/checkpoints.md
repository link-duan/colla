# Checkpoints and restoration

HistoryCheckpoint preserves undo/redo stacks, grouping, capacity and the exact content
basis required to interpret its stored changes. A content-only Value is insufficient
to resume undo history.

## Standalone restoration

```ts
import { Document, History, HistoryCheckpoint, Value } from 'colla-ot'
const doc = Document.create({ count: 0n })
const history = History.attach(doc)
doc.edit(tx => tx.increment(['count'], 1n))
const contentBytes = doc.snapshot().encode()
const historyBytes = history.checkpoint().encode()
history.close()
doc.close()
const restored = Document.create(Value.decode(contentBytes))
const undo = History.restore(restored, HistoryCheckpoint.decode(historyBytes))
undo.undo()
console.log('Undo after restoration:', restored.get(['count'])?.toJS()) // 0n
undo.close()
restored.close()
```

Store the matching content and history atomically. Restoration validates their exact
basis, including IDs; independently reconstructed content that only looks equal is
not sufficient. A mismatch is an error, not an invitation to silently reset history.

## Synchronized restoration

A SessionCheckpoint includes enabled History along with pending requests and the visible
content basis. Restore the session, then use History.attach(session.document) to obtain
the restored History. Stop the previous writer before resuming the same client ID.
This resumes undo tracking together with the original session’s unconfirmed work.

Keep standalone checkpoints separate from server log retention. See
[Persistence](/docs/production/persistence) for a comparison of all durable objects.
