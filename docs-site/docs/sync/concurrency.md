# Concurrency and conflicts

Concurrent changes are interpreted against a common immutable Value basis. Authority
establishes one ordering for the document; clients rebase optimistic work as that ordered
history arrives.

## Mergeable intent

Text insertions, deletions and RichText formatting are transformed with explicit priority
where intent competes. Content edits to a moved element follow its ID. Same-source
competing Moves are resolved by priority while retaining mergeable content edits.
Independent moves are not rejected merely because both are structural operations.

## Structural conflict boundaries

A deletion of an element or its ancestor in the common base wins over a competing escape
Move. Merging changes that creates an ownership cycle, loses a destination parent or
cannot safely resolve a Map key occupancy produces structural_conflict. Colla will not
silently delete an unrelated element to make a merged result fit.

Treat these failures as recovery conditions when receiving ordered synchronization work.
Preserve pending intent and expose an explicit reconciliation path. Do not retry an
invalid structural merge indefinitely or hide it as a successful Noop.

## Guarantees and limits

For documented mergeable cases, transform satisfies TP1: applying either concurrent
branch and then its transformed counterpart converges to the same content and IDs.
This is not a TP2 guarantee or a general peer-to-peer protocol. Use the centralized
ordering model and distinguish algorithm properties from your transport's ordering,
durability and retry guarantees.

See [Change algebra](/docs/core/changes) for the return order and a runnable concurrent-edit example.
