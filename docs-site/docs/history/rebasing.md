# Remote changes and rebasing

History stores inverse Changes and rebases them when remote work arrives. This preserves
local undo intent without replacing the document with an old snapshot.

## Remote work takes priority

Remote changes rebase both undo and redo with remote priority. History skips inverse
work that has become a Noop. A new local edit clears redo; remote editing does not simply
discard the redo stack. A remote commit also ends the current explicit edit group.

For additive Int edits, if Alice contributes +1 and Bob contributes +10, Alice's undo
removes her +1 while keeping Bob's +10. The [collaborative undo example](/docs/examples/history)
shows this through an Authority and also synchronizes redo.

## Container deletion is still deletion

Undoing the insertion of a parent container deletes that container, including content
another user subsequently inserted inside it. Do not describe collaborative undo as
always preserving every later remote edit. If your product requires confirmation for
such destructive user intent, implement that interaction in the application.

## Synchronization behavior

Undo/redo produce ordinary local edits, enter the session's outbound pipeline and receive
formal Commits. They do not cancel an old request or reuse its sequence number. Continue
sending and receiving through the same SyncSession while History is attached.

If synchronization enters recovery-required, preserve the session checkpoint and exported
content. Starting from a fresh server baseline is an explicit recovery decision; a
History checkpoint from another content basis cannot simply be attached to it.
