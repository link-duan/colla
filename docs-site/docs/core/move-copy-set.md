# Move, Copy and Set

Choose an operation according to whether the element should remain the same instance.
Identity affects references, concurrent edits and undo as well as path lookup.

| Operation | Target identity | Descendant identities | Internal Refs |
| --- | --- | --- | --- |
| Move | Preserved | Preserved | Targets unchanged |
| Copy | New | New | Remapped inside copied subtree |
| Set existing | Preserved | Newly imported | Imported root maps to retained target |

## Move an existing element

`tx.move(source, { parent, key })` targets a Map; `{ parent, index }` targets a List.
Source and parent accept Paths or IDs. The destination parent must exist and match
the slot kind; a Map key must be vacant.

List destination indexes are interpreted **after removing the source**. Moving `A`
from index 0 to index 2 in `[A, B, C]` produces `[B, C, A]`. A same-position Move is a
Noop. The document root cannot move, and an element cannot move into itself or a descendant.

## Copy a subtree

`tx.copy(source, destination)` uses the same destination shape and returns the new
root ID. Its source remains in place, so its destination index uses the current List
without subtracting a removed item. Map destinations must be vacant.

The source may be the document root. This copies the current tree into an existing
archive List; the copy contains its own empty archive, not a recursive reference:

<<< ../../examples/root-copy.ts

All owned IDs in the copied tree are new. Internal Refs follow the corresponding copied
elements; references to elements outside the copied subtree keep their original targets.

## Set a value

`tx.set(location, input)` replaces an existing target while retaining its ID. Imported
descendants get fresh IDs. Set can also add a Map's final key, but does not create
missing intermediate parents or append a List item. See [Map and List editing](/docs/editing/maps-lists).

## Concurrent edits and undo

Content edits follow moved IDs. See [Concurrency and conflicts](/docs/sync/concurrency)
for competing Moves, ancestor deletion and ownership conflicts. Undo restores the
identities removed by the original edit; see [References](./references) for dangling
Ref behavior and [Move and Ref](/docs/examples/move-ref) for a complete example.
