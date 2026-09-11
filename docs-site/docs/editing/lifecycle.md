# Runtime lifecycle

Document, History and SyncSession are runtime objects. Close them when their editing
or synchronization work ends. Immutable Values, Changes, checkpoints and events have
no public dispose/free lifecycle.

## Ownership and cleanup

A SyncSession exposes its Document. Closing the session closes that Document and clears
its session listeners. Closing a standalone History detaches undo tracking; it does not
replace application cleanup of the Document. Close operations are idempotent.

Read and write operations on a closed Document fail. Take any snapshot or checkpoint
you need before closing. Already returned immutable data remains valid afterward, so
background storage or rendering may finish using that data.

## Transaction and event boundaries

Do not close a Document during its edit callback or while dispatching an event. Do not
hold a Transaction or its editors across callbacks. Their validity is scoped to one
synchronous edit, not to the lifetime of the Document.

## Session restart

A restored session resumes its original client ID. Stop the old writer before restoring
so two instances cannot generate requests with the same identity. Remove transport
handlers and stop scheduled sends as well as closing the session.

Take the checkpoint before closing. The [restart example](/docs/production/persistence#client-durability)
shows the sequence; [Recovery](/docs/sync/recovery) covers unavailable history separately.
