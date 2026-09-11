# Integration testing

Test the library contract and your integration separately. Colla's algebra and codec
tests cannot establish that your application broadcasts only durably stored commits or
restores the correct client's checkpoint.

## What library tests establish

Algebra properties and cross-language codec fixtures check library behavior. They do
not establish your application's delivery, durability or editor behavior. See
[Changes and OT algebra](/docs/core/changes#guarantees-and-boundaries) for the supported
convergence contract.

## Integration scenarios

Exercise simultaneous clients, buffered edits, duplicate requests, lost replies, delayed
Commits, missing revisions, client and server restarts, and history compaction. Assert
both visible content and IDs converge. Verify that retry bytes survive a restart and
that a persistence failure never causes an uncommitted server state to be broadcast.

For editors, test emoji boundaries, rich-text embeds, selection after Move, IME composition,
listener failures and feedback suppression. For recovery, verify local work is retained,
sending stops, and the application offers an explicit reconciliation path.
