# SyncSession

SyncSession coordinates a confirmed SyncSnapshot, the client's visible Document, one
in-flight request and one composed buffer of subsequent edits. Create it only from a
controlled SyncSnapshot, never from an arbitrary content Value.

## Create and edit

```ts
import { Authority, SyncSession } from 'colla-ot'
const authority = Authority.create({ documentId: 'demo', value: { count: 0n } })
const session = SyncSession.create({ clientId: 'writer-1', snapshot: authority.snapshot() })
session.document.edit(tx => tx.increment(['count'], 1n))
console.log('Visible local count:', session.document.get(['count'])?.toJS()) // 1n, already visible
console.log('Confirmed server revision:', session.revision) // 0n, not confirmed yet
console.log('Submission base revision:', session.outbound()?.baseRevision) // 0n
session.close()
```

The application's transport delivers `outbound().encode()` and calls receive with a
decoded ServerMessage. Calling outbound only inspects available work; it does not mark
a request sent, acknowledged or eligible for a new sequence number.

## State and subscriptions

`state` contains status (active, recovery-required or closed), confirmed revision,
hasOutbound and optional recoveryReason. Subscribe to state transitions when scheduling
sends and updating indicators. Content changes are observed separately on the Document.

## Writer identity and restart

Use one active writer per client ID. A reconnect keeps the existing session; a process
restart restores its SessionCheckpoint. Do not create a replacement session from the
visible Value or assign a fresh identity to resend a lost request.

See [Runtime lifecycle](/docs/editing/lifecycle#session-restart) for stopping an old writer,
[Persistence](/docs/production/persistence#client-durability) for checkpoint restoration,
and [Retries](./retries) for stable request delivery.
