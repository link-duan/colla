# Editing, history and collaboration

Rust owns Document, Transaction, History, SyncSession and immutable Authority
state transitions. JavaScript performs input conversion, synchronous scope
management and isolated subscriptions. Content, local version, History, pending
and outbound effects are computed before a single commit.

A scoped transaction can read its working Value, edit every value type, move or
copy subtrees and apply an identity-explicit Change. Normalized Noop returns no
result and changes no runtime state. Closing or receiving remotely inside the
scope fails. Escaped JavaScript editors are invalid, including read access.

EditResult preserves before/after, change/inverse, replayable scalar edit steps,
local version and origin. Move steps retain their source ID. Events occur after
commit; reads are allowed during dispatch, but reentrant edits are rejected.
Listener exceptions cannot roll back content or interrupt other listeners.

History saves inverse Changes. Remote changes rebase undo and redo with remote
priority. Default capacity is 100 explicit undo units. Consecutive same-group
transactions merge; remote commits and undo/redo break groups. Undo/redo are new
local changes and therefore participate in synchronization. Deleting an inserted
container during Undo also deletes subsequent content inside it.

A SyncSession has a confirmed SyncSnapshot, an original in-flight Submission,
its rebased working Change and one composed buffer. Original retry identity and
bytes never change. The client's Commit both confirms the request and promotes
its buffer. Authority.accept returns a new Authority and message, allowing the
application to persist before adopting state. Retained logs supply content bases
for transformations. Compaction advances the available history floor while
preserving deduplication receipts.

Duplicate messages are idempotent. Revision gaps leave content unchanged and
identify the missing range. Structural rejection, expired history or impossible
rebase enters recovery-required, retaining all work and permitting local edits
and exports while stopping outgoing synchronization. Applications establish a
fresh session and merge retained work explicitly.

Value, SyncSnapshot, SessionCheckpoint, HistoryCheckpoint and AuthorityCheckpoint
have different persistence responsibilities. Session checkpoints include enabled
History. All controlled objects use strict Rust v2 codecs. See the
[public API](https://link-duan.github.io/colla/reference/javascript) and [binary format](binary-format.md).
