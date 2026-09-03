---
title: Protocol boundaries
description: How Colla Core bodies and Document envelopes fit into an application protocol.
---

# Protocol boundaries

<p class="lead">Colla defines canonical content and operation bytes; your application defines how those bytes are identified, ordered, authenticated, and recovered.</p>

## Body versus envelope

| Format | Contents | Typical use |
| --- | --- | --- |
| Value body | One Core Value | Pure persistence or transport payload |
| Change body | One Core Change | Pure operation payload |
| Snapshot (`COLLAS`) | Protocol version, revision, complete Value | Content checkpoint |
| Update (`COLLAU`) | Protocol version, base revision, local `updateId`, Change | Application transport unit |

The Value and Change bodies are shared by Rust and JavaScript. A Snapshot or
Update adds local envelope metadata; it is not a new OT operation kind.
`updateId` starts at `1` for each JavaScript `Document` instance and correlates
local FIFO acknowledgements. It is not a globally unique operation identity and
is not restored from a Snapshot.

## Strict decoding

Treat every byte crossing a process or trust boundary as untrusted. Matching
decoders reject:

- incorrect `COLLAS`/`COLLAU` magic or unsupported protocol versions;
- truncated or malformed tuples, unknown tags, and invalid UTF-8;
- non-minimal varints and invalid canonical ordering;
- trailing bytes after the complete body or envelope.

The codec has built-in recursion and allocation defenses for byte input.
Structured JavaScript input has separate `InputOptions` limits for depth, node
count, container length, string bytes, sequence operations, and sequence length.
Those limits do not change valid Core algebra semantics.

## What the application protocol adds

An application protocol should define, outside the Colla envelope:

- document identity, tenant scope, and authentication/authorization;
- request identity, retry and deduplication behavior;
- durable server revision order and accepted-operation history;
- compression, checksums, quotas, and rate limits when required;
- replay, Snapshot fallback, and protocol version migration policy.

The server must order accepted Changes. A client should send the Update's base
revision, apply only the next confirmed remote revision, and request replay or
a new Snapshot after a gap. Keep acknowledgements separate from editor
rendering: an ack changes confirmed state but does not create a visible edit.

## Persistence and recovery

A Snapshot is a content checkpoint, not a resumable collaboration session. It
contains visible content and revision, but no pending queue, listeners, retry
state, or transport connection. If local delivery must survive a crash, persist
an outbound Update queue and application protocol state beside the Snapshot
with an atomic recovery boundary.

```text
checkpoint = { snapshotBytes, pendingUpdateBytes[], protocolState }
```

On recovery, validate every envelope before rebuilding the queue. Preserve FIFO
order, discard acknowledged entries only after durable confirmation, and never
guess a missing revision or substitute placeholder content for a rejected
Change.

## Scope boundary

Core does not provide a session, network transport, global deduplication,
presence, cursor tracking, editor rendering, or a product-specific history
format. The [Document state](/docs/document/) manages one process's visible and
confirmed state; the surrounding protocol remains responsible for delivery
guarantees.

Next: [Document local and remote updates](/docs/document/local-remote),
[production sync protocol](/docs/production/sync-protocol), and
[errors and limits](/docs/production/errors-limits).
