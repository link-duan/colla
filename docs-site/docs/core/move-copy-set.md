# Move, Copy and Set

| Operation | Effect                                                | Concurrent edits inside the element |
| --------- | ----------------------------------------------------- | ----------------------------------- |
| ListMove  | Moves one element within its List                     | Follow the element                  |
| Copy      | Inserts the source content at a vacant destination    | Apply to the source only            |
| Set       | Replaces an existing element, or adds a final Map key | Discarded by the replacement        |

## Move within a List

`tx.list(path).move(from, to)` moves the element at `from`. The destination index is
interpreted **after removing the element**. Moving `A` from index 0 to index 2 in
`[A, B, C]` produces `[B, C, A]`. A same-position move is a Noop.

There is no cross-parent move. Delete the element and insert it at the destination;
concurrent edits to the original element do not follow it to the new location.

## Copy a subtree

`tx.copy(source, destination)` inserts the source content at the destination Path. Its
final segment is a vacant Map key or a List insertion index; the source remains in
place, so the index uses the current List.

The source may be the document root. This copies the current tree into an existing
archive List; the copy contains its own empty archive, not a recursive reference:

<<< ../../examples/root-copy.ts

## Set a value

`tx.set(path, input)` replaces an existing target. Set can also add a Map's final key,
but does not create missing intermediate parents or append a List item. See
[Map and List editing](/docs/editing/maps-lists).

## Concurrent edits and undo

See [Concurrency and conflicts](/docs/sync/concurrency) for how ListMove, Delete and
Set combine with concurrent edits, and [ListMove](/docs/examples/list-move) for a
complete example. Undo restores the content removed by the original edit.
