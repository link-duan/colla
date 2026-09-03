---
title: Events and lifecycle
description: Typed Document events, listener isolation, and teardown.
---

# Events and lifecycle

`Document.on()` accepts exactly `change` or `error` and returns an unsubscribe
function. Register listeners before applying edits and keep each unsubscribe
function with the editor or service that owns it.

## Change events

```ts
// `editor`, `outboundQueue`, `metrics`, and `report` are application adapters.
const stop = document.on('change', event => {
  if (event.origin === 'local') {
    outboundQueue.push(event.revision)
    return
  }
  editor.applyEditSteps(event.editSteps)
})
```

Every change event is emitted after the state transition commits:

| Field | Contract |
| --- | --- |
| `origin` | `'local'` or `'remote'`. |
| `revision` | Visible revision after this transition. |
| `editSteps` | Frozen editor projection of the visible Change, relative to the previous value. |

`editSteps` is not a Core Change, persistence envelope, or transport message.
It contains paths and editor-facing operations, including UTF-16 positions where
appropriate. The event does not expose the internal owned Change handle.

## Error events

```ts
document.on('error', ({ error }) => {
  metrics.increment('document.listener_error')
  report(error)
})
```

If a change listener throws, the Document remains committed, remaining change
listeners still run, and the thrown value is delivered to error listeners (or
logged to `console.error` if no error listeners are registered).
Error-listener failures are swallowed and never recursively reported. Errors
from `applyLocal`, `applyRemote`, or `ack` are operation failures instead;
they are thrown to the caller and do not become events.

## Teardown rules

- Do not register listeners inside a render loop.
- Apply each remote event exactly once in the editor adapter.
- Unsubscribe before replacing a Document instance.
- Dispose the Document after its listeners and owned envelopes are released.
- Treat event payloads as read-only snapshots.

Next: [Editor integration](/docs/document/editor-integration).
