---
title: Document lifecycle
description: Construct, edit, synchronize, checkpoint, and dispose a Document.
---

# Document lifecycle

<p class="lead">A Document has no hidden connection lifecycle. The application explicitly creates it, feeds it ordered Updates, acknowledges local work, and disposes it.</p>

## Recommended sequence

1. Create with `Document.fromJS(value, revision)` or restore with
   `Document.fromSnapshot(snapshot)`.
2. Subscribe to `change` and `error` before the first edit.
3. Apply local Core Changes with `applyLocal()` and enqueue the returned Update.
4. Feed server-ordered Updates to `applyRemote()`.
5. Call `ack(updateId)` only for the oldest accepted local Update.
6. Write `snapshot()` when a content checkpoint is needed.
7. Unsubscribe views and call `dispose()` when the session ends.

```ts
import { Document } from 'colla-ot'

const document = Document.fromJS('Draft', 42n)
// `logger` and `storage` are application-provided services.
const stop = document.on('error', ({ error }) => logger.warn(error))

const checkpoint = document.snapshot()
await storage.write('document.snapshot', checkpoint.encode())
checkpoint.dispose()

stop()
document.dispose()
```

## Transition semantics

```text
create/restore
      │
      ├─ applyLocal ── visible changes immediately; queue Update
      ├─ applyRemote ─ confirmed revision checked; pending changes rebased
      ├─ ack ───────── oldest pending Update becomes confirmed
      └─ snapshot ──── copy visible value + visible revision
```

`applyLocal()` advances visible revision and emits one local change event. It
does not mean the server has accepted the edit. `applyRemote()` advances both
visible and confirmed revisions after applying the Update and rebasing pending
local Changes with the fixed `left-first` tie-break. `ack()` emits no change
event because acknowledgement changes confirmation, not visible content.

## Disposal

`dispose()` is idempotent. It releases visible and confirmed value handles,
pending Update and Change handles, and listeners. `Document`, `Snapshot`,
`Update`, `ValueHandle`, and `Change` also support `Symbol.dispose`. Dispose
owned snapshots, updates, and values in long-running or high-allocation code.

Next: [Events and lifecycle](/docs/document/events-lifecycle).
