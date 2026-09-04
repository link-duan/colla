# JavaScript: Document synchronization

```ts
import { Document, text } from 'colla-ot'

const doc = Document.fromJS(text('ab'), 0n)
const unsubscribe = doc.subscribe(event => {
  console.log(event.origin, event.revision, event.editSteps)
})

const pending = doc.transact(tx => {
  tx.text([], t => t.retain(1).insert('X'))
})

await transport.send(pending.bytes)
doc.applyRemote(await transport.receive())
doc.ack(pending.updateId)
```

The visible content immediately becomes `aXb`.
If a remote Update based on revision 0 inserts `Y` at the same position,
Document uses left-first rebasing and the result is `aXYb`.
`ack()` does not emit a change event.
Production code owns connection state.
Production code owns retries and the outbound queue.
Production code handles decode failures.
Production code handles revision mismatches.
Production code maps `CollaError` to protocol responses.

## Application notes

The editor displays local content immediately.
The transport sends canonical Update bytes (`update.bytes`).
The server orders accepted Changes.
The client accepts the next confirmed revision.
The Document rebases pending local work.
The editor receives one remote projection.
Acknowledgements are cumulative up to the accepted updateId.
The local update ID is instance-local.
The server uses its own request identity.
Retries belong to the outbound queue.
Reconnect logic belongs to the controller.
Snapshot fallback belongs to the application protocol.
Malformed bytes are rejected before state changes.
Revision gaps trigger replay or resync.
Listener failures are reported separately.
Dispose the Document during session teardown.

## Next

Read [Core JavaScript](./javascript-core).
Read [Document lifecycle](../document/lifecycle).
Read [production sync](../production/sync-protocol).
