# Map and List editing

Use Map keys for named fields and List indexes for ordered items. Make related changes
inside one `doc.edit` callback so they commit together. See [Transactions](./transactions)
for rollback and callback scope.

## Add, replace and remove

<<< ../../examples/maps-lists.ts

`set` creates a missing final Map key or replaces an existing value. `delete` requires
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

## Copy a subtree

`tx.copy(source, destination)` inserts the current content at `source` into the
destination Path, whose final segment is a vacant Map key or a List insertion index.
The source stays in place. It may be the root: this copies the current tree into an
existing archive List, and the copy contains its own empty archive rather than a
recursive reference:

<<< ../../examples/root-copy.ts

How ListMove, copies, Set and deletion behave under concurrent edits is compared in
[Concurrent edits](/docs/core/concurrency#moving-copying-and-replacing).
