# Document and snapshots

Document owns visible content and local editing state. Create it with ordinary input
or a Value. A Value returned by `snapshot()` is immutable and can outlive the Document.
For synchronized work, edit `session.document` instead of creating a separate Document.

## Read current or frozen content

```ts
import { Document } from 'colla-ot'
const doc = Document.create({ count: 0n })
const frozen = doc.snapshot()
doc.edit(tx => tx.increment(['count'], 1n))
console.log('Frozen snapshot:', frozen.get(['count'])?.toJS()) // 0n
console.log('Current document:', doc.get(['count'])?.toJS()) // 1n
console.log('Local content version:', doc.version) // 1n
doc.close()
```

Document and Transaction expose the same read operations as Value: get, has, kind,
idAt, pathOf, resolve and referencesTo. Transaction reads observe its working edits.
Value reads always observe that Value's snapshot.

## Local version is not server revision

A successful non-Noop content commit advances local `version`, including remote content
changes and undo/redo. A Noop edit returns null and does not emit a content event.
SyncSession's revision tracks server confirmation separately: confirmation can progress
without another visible content edit. Never send Document.version as a protocol revision.

## Persistence choice

`Document.create(Value.decode(bytes))` restores standalone content and IDs, not a
synchronized session or its History. Use SessionCheckpoint to resume pending work and
HistoryCheckpoint for standalone history. See [Persistence](/docs/production/persistence).
Continue with [Transactions](./transactions) to edit atomically.
