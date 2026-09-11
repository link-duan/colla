# Collaborative undo

Alice and Bob contribute to a shared counter. Alice then undoes her own edit after receiving Bob’s change. The same exchange function sends undo and redo through the normal synchronization flow.

## Example

<<< ../../examples/history.ts

## Expected behavior

Undo leaves Bob’s `10n` contribution intact. Redo restores Alice’s contribution and brings both clients to `11n`. Neither operation rewinds the server revision.

## Use it in your application

Attach History to the Document owned by the session. Send the resulting undo and redo edits just like ordinary local edits. The transport here is in memory; persist Authority state before broadcasting in production. Read [Grouping](/docs/history/grouping) to combine a gesture’s edits and [Checkpoints](/docs/history/checkpoints) to preserve undo across restart.
