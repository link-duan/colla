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

Replacing an existing value with `set` retains that element's ID. List `replace` removes
the selected items and imports replacements with fresh IDs. To reorder an existing
item while keeping its identity and references, use [Move](/docs/core/move-copy-set).
