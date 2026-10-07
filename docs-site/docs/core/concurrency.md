# Concurrent edits

When two Changes are made against the same content, [transform](./algebra) rewrites
each operation's Path and positions across the other side's edits, so an edit lands on
the element it was made against. Every combination produces a result; there is no
conflict state for the application to resolve.

## How concurrent edits resolve

| Concurrent edits                              | Result                                                         |
| --------------------------------------------- | -------------------------------------------------------------- |
| Delete or Set an element + any edit inside it | Delete/Set wins; the inner edit is discarded                   |
| Delete + Set of the same element              | Delete wins                                                    |
| Set + Set of the same element                 | Priority selects the value, also when both create a Map member |
| Insert at the same List position              | Both are kept; priority's item comes first                     |
| ListMove + edit inside the moved item         | The edit follows the item                                      |
| ListMove + Delete of the moved item           | Delete wins                                                    |
| ListMove + ListMove of the same item          | Priority selects the destination                               |
| Text or RichText edits on the same value      | Both apply; priority orders insertions at the same position    |
| Add + Add on the same Int                     | Both additions apply                                           |
| Add + Set on the same Int                     | Set wins                                                       |

Priority is the left/right rule passed to `transform`; see
[Change algebra](./algebra#priority) for how the runtime chooses it.

Within one List, each insertion or move destination is anchored before its right
neighbor. If the other side deletes or moves that neighbor, the anchor becomes the next
item that stays in place.

## Moving, copying and replacing

Four ways to relocate, copy or replace content behave differently under concurrency:

| Edit                         | Effect                                 | Concurrent edits inside the original element |
| ---------------------------- | -------------------------------------- | -------------------------------------------- |
| ListMove                     | Reorders one item within its List      | Follow the item                              |
| Delete + write               | Moves content to another parent        | Discarded with the deleted element           |
| Write a Value read elsewhere | Copies existing content to a new place | Apply to the original only                   |
| Set                          | Replaces an element                    | Discarded by the replacement                 |

Only ListMove preserves concurrent work while relocating content, and only within one
List. There is no cross-parent move; moving content to another parent is a deletion
followed by a write. A movable tree type with cross-parent moves is planned as an
opt-in addition. The [ListMove example](/docs/examples/list-move) shows a reorder and a
concurrent edit converging.

## Model content to preserve work

Deletion and replacement discard concurrent work inside the affected element. Where
losing that work matters:

- Edit the smallest element that changes. Setting one Map field keeps concurrent edits
  to sibling fields; replacing the whole Map discards them.
- Use Text for fields users type in, so concurrent typing merges instead of competing
  through Set.
- Reorder List items with ListMove instead of deleting and reinserting them.
- Use Int with Add for counters, so concurrent increments accumulate.
