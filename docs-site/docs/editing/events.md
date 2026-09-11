# Subscriptions and events

Document subscriptions report successful content commits. Subscribe once when attaching
an editor view and call the returned unsubscribe function when detaching it.

## Observe content

```ts
import { Document } from 'colla-ot'
const doc = Document.create({ count: 0n })
const unsubscribe = doc.subscribe(event => {
  console.log('Content commit:', event.origin, event.version, event.after.get(['count'])?.toJS())
}, { onError: error => console.error('View update failed', error) })
doc.edit(tx => tx.increment(['count'], 1n)) // local, 1n, 1n
unsubscribe()
doc.close()
```

Callbacks run synchronously after commit. Listener failures are isolated and do not
roll back the committed content or stop other listeners. Supply onError to report view
failures. Immutable events can safely be retained by asynchronous rendering work.

## Reentrancy and feedback loops

Reading during dispatch is allowed; mutation is rejected. Schedule a later task if a
listener must trigger another edit. Keep that task conditional to avoid a feedback loop.
Do not send programmatic UI updates back as new user edits. Use origin and an adapter
suppression mechanism to distinguish model rendering from fresh input.

## Observe synchronization separately

`session.subscribe` reports status, confirmed revision, hasOutbound and recoveryReason.
It can fire without a content event, including acknowledgement-only progress. Use this
channel for sending availability and sync indicators; it has the same listener isolation
and mutation restrictions. See [SyncSession](/docs/sync/session).
