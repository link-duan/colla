# Map and List editing

Use Map keys for named fields and List indexes for ordered items. Make related changes
inside one `doc.edit` callback so they commit together. See [Transactions](./transactions)
for rollback and callback scope.

## Add, replace and remove

<<< ../../examples/maps-lists.ts

`set` writes a Map member, creating it when missing, or replaces an existing List item. `delete` requires
an existing target. Neither operation creates missing intermediate parents.

The List editor takes arrays for inserted or replacement items. Indexes and removal
counts refer to the working List at each call. In this example, inserting `review`
shifts `publish` to index 2 before replacement.

## Choose the right operation

Use `list.insert(listLength, values)` to append. `set(['steps', listLength], value)`
does not append, and an out-of-range removal fails the whole transaction.

List `replace` removes the selected items and inserts the replacements; concurrent edits
to the removed items are discarded. To reorder an existing item so that concurrent
edits follow it, use `list.move`.

## Move within a List

`tx.list(path).move(from, to)` moves the item at `from`. The destination index is
interpreted **after removing the item**: moving `A` from index 0 to index 2 in
`[A, B, C]` produces `[B, C, A]`. A same-position move is a Noop. There is no
cross-parent move; delete the item and insert it at the destination instead.

## Copy content

Values are immutable and carry no identity, so copying is reading a Value and writing
it elsewhere. The source stays in place:

```ts
import { Document } from 'colla-ot'
const doc = Document.create({ title: 'Draft', archive: [] })
doc.edit(tx => tx.list(['archive']).insert(0, [tx.get([])!]))
console.log('Archived title:', doc.get(['archive', 0, 'title'])?.toJS()) // Draft
console.log('Archive inside copy:', doc.get(['archive', 0, 'archive'])?.toJS()) // []
doc.close()
```

The copy holds the content as it was read: copying the root into the archive stores the
earlier, empty archive, not a recursive reference.

How ListMove, copies, Set and deletion behave under concurrent edits is compared in
[Concurrent edits](/docs/core/concurrency#moving-copying-and-replacing).
