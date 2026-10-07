# Introduction

Colla is a library for structured documents that several people edit at the same time.
It gives your application an editable content tree, undo and redo that respect other
users' work, and the client and server state machines that keep everyone's copy in
step. You provide the network, storage and user interface.

## What you build with it

Colla fits applications whose content is more than a single text field: task boards,
forms, outlines, design tools, notes with structured blocks. A document is a tree of
Maps, Lists, scalars and collaborative Text or RichText. Users can edit any part of it
concurrently, including reordering List items and typing in the same paragraph, and
every client converges to the same content.

## The building blocks

| Piece                             | Role                                                                    |
| --------------------------------- | ----------------------------------------------------------------------- |
| [Value](/docs/core/values)        | Immutable content tree; also the format for snapshots and storage       |
| [Document](/docs/editing/)        | Editable content in one place; edits commit atomically and emit events  |
| [History](/docs/history/)         | Undo and redo of local edits, rebased across remote edits               |
| [SyncSession](/docs/sync/session) | Client side of synchronization: optimistic edits, pending work, retries |
| [Authority](/docs/sync/authority) | Server side: orders submissions into one sequence of commits            |

You can adopt them in that order. A Document on its own is a complete local editor
model; History adds undo; SyncSession and Authority add collaboration.

## How collaboration works

Colla uses centralized Operational Transformation. Each client applies its own edits
immediately, then submits them to an Authority on your server. The Authority puts all
submissions into a single order and returns commits, which clients apply on top of
their own pending work. Concurrent edits are transformed rather than rejected, so
nothing has to be resolved by hand. The rules are described in
[Concurrent edits](/docs/core/concurrency).

Because one Authority orders every document, Colla is not a peer-to-peer or CRDT
library. If clients must merge with each other without a server, choose a different
tool.

## What your application provides

Colla is synchronous and does no I/O. It never opens a socket, schedules a timer or
writes to disk. Your application owns:

- transport between clients and the server, and when to send;
- persistence of server state and client checkpoints;
- authentication, authorization and validation of what users may change;
- the editor view, selection, presence and cursors.

Colla gives you encoded bytes to send and store, and decoders that reject anything
invalid. [Production](/docs/production/persistence) covers each of these boundaries.

## Next steps

[Install the package](./installation), then follow the [tutorial](./tutorial) to build a
shared task list from a local Document to two synchronized clients.
