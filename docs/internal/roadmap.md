# Colla delivery scope

The active delivery is the [0.4.0 implementation contract](../implementation-0.4.0.md):
stable identities, native Move and Ref, complete transactional editing, History,
centralized SyncSession/Authority, recovery and strict version-2 codecs.
The project is in development and has no official release yet.

Transport, storage engines, authentication, presence, editor-specific adapters,
cross-document references and arbitrary peer-to-peer synchronization remain
application concerns. A future scope decision must preserve the small-artifact
constraint and record any dependency/format tradeoff explicitly.

Publishing is separate from implementation and follows the coordinated release
runbook only after all required validation and explicit release authorization.
