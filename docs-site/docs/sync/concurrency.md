# Concurrency and conflicts

Concurrent changes are interpreted against a common immutable Value basis. Authority
establishes one ordering for the document; clients rebase optimistic work as that ordered
history arrives.

## How concurrent edits resolve

Transform rewrites each operation's Path across the other side's insertions, deletions
and List moves, so an edit lands on the element it was made against. Text insertions,
deletions and RichText formatting are transformed with explicit priority where intent
competes. Every combination produces a result:

| Concurrent edits                               | Result                                  |
| ---------------------------------------------- | --------------------------------------- |
| Delete or Set an element, edit inside it       | Delete/Set wins; the inner edit is lost |
| Delete and Set the same element                | Delete wins                             |
| Set the same element twice                     | Priority selects the value              |
| Insert the same new Map key                    | Priority selects the value              |
| Insert at the same List position               | Priority's item comes first             |
| Move a List item, edit inside it               | The edit follows the item               |
| Move and delete the same List item             | Delete wins                             |
| Move the same List item to different positions | Priority selects the position           |

A deletion or replacement discards concurrent work inside that element. Where losing
such work matters, prefer finer-grained edits over replacing a whole subtree.

## Guarantees and limits

Transform satisfies TP1: applying either concurrent
branch and then its transformed counterpart converges to the same content.
This is not a TP2 guarantee or a general peer-to-peer protocol. Use the centralized
ordering model and distinguish algorithm properties from your transport's ordering,
durability and retry guarantees.

See [Change algebra](/docs/core/changes) for the return order and a runnable concurrent-edit example.
