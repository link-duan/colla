# Colla delivery scope

Colla provides stable identities, native Move and Ref, atomic editing, History,
centralized SyncSession/Authority, recovery and strict Rust codecs. Current
contracts are defined by the [specifications](../README.md).

Transport, storage engines, authentication, presence, editor-specific adapters,
cross-document references and arbitrary peer-to-peer synchronization remain
application concerns. Scope changes must preserve the
[artifact constraints](documentation.md#artifact-constraints) and record meaningful
dependency or format tradeoffs.

Publishing follows the [coordinated release runbook](releasing.md) only after all
required validation and explicit release authorization.
