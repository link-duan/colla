---
title: Document state
description: The application-facing state machine for visible content and ordered updates.
---

# Document state

<p class="lead">`Document` turns immutable values and changes into a small, explicit editing state machine.</p>

Use `Document` when an application needs visible optimistic edits, server
revision tracking, pending local Updates, and typed events. Use the immutable
Value and Change operations directly when the caller owns document state.

## The complete loop

```text
Core Value + confirmed revision
             │
             ├─ transact(tx => ...) ──> visible value + pending Update
             │                                   │
             │                                   └─ application transport (update.bytes)
             │
             ├─ applyRemote(bytes | Update) ──> rebase pending locals
             │                                   │
             │                                   └─ subscriber (remote editSteps)
             │
             └─ ack(updateId) ───────────────> confirmed revision advances (cumulative)
```

```ts
import { Document, text } from 'colla-ot'

const document = Document.fromJS(text('Draft'))
// `editor` and `transport` are application-provided adapters.
const stop = document.subscribe(event => {
  if (event.origin === 'remote') editor.applyEditSteps(event.editSteps)
})

const outbound = document.transact(tx => {
  tx.text([], t => t.retain(5).insert(' v2'))
})
await transport.send(outbound.bytes)
document.applyRemote(await transport.receive())
document.ack(outbound.updateId)

stop()
document.dispose()
```

## Responsibility boundaries

| Document owns | The application owns |
| --- | --- |
| Visible and confirmed Core values | Network transport and authentication |
| Local FIFO Update IDs | Retry, queue durability, and request IDs |
| Revision checks and OT rebasing | Server ordering and persistence policy |
| Change and error events | Editor rendering and product schema |

`Document` does not implement sessions, permissions, presence, cursors, global
deduplication, or server history. Continue to [state](./state), then read
[local and remote updates](./local-remote) and [events](./events-lifecycle).

Next: [Lifecycle](/docs/document/lifecycle).
