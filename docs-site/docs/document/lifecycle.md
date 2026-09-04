---
title: Document lifecycle
description: Construct, edit, synchronize, checkpoint, and dispose a Document.
---

# Document lifecycle

<p class="lead">A Document has no hidden connection lifecycle. The application explicitly creates it, feeds it ordered Updates, acknowledges local work, and disposes it.</p>

## Recommended sequence

1. Create with `Document.fromJS(value, revision)` or restore with
   `Document.fromSnapshot(snapshot)`.
2. Subscribe to change events and error handling with `document.subscribe(...)`.
3. Apply atomic mutations with `document.transact(...)` and enqueue `update.bytes`.
4. Feed server-ordered Updates or raw bytes to `applyRemote()`.
5. Call `ack(updateId)` when server accepts local work (cumulative acknowledgement).
6. Write `document.snapshot().bytes` when a content checkpoint is needed.
7. Unsubscribe views and call `dispose()` when the session ends.

```ts
import { Document } from 'colla-ot'

const document = Document.fromJS('Draft', 42n)
// `logger` and `storage` are application-provided services.
const stop = document.subscribe({
  onError: ({ error }) => logger.warn(error),
})

const checkpoint = document.snapshot()
await storage.write('document.snapshot', checkpoint.bytes)

stop()
document.dispose()
```

## Transition semantics

```text
create/restore
      │
      ├─ transact ──── visible changes immediately; returns pure Update
      ├─ applyRemote ─ confirmed revision checked; pending changes rebased
      ├─ ack ───────── pending Updates up to ID become confirmed (cumulative)
      └─ snapshot ──── create pure Snapshot { revision, bytes, value }
```

`transact()` advances visible revision and emits one local change event. It
does not mean the server has accepted the edit. `applyRemote()` advances both
visible and confirmed revisions after applying the Update and rebasing pending
local Changes with the fixed `left-first` tie-break. `ack()` emits no change
event because acknowledgement changes confirmation, not visible content.

## Disposal

`dispose()` is idempotent. It releases internal Wasm value handles and cleans up
subscribers. `Document`, `ValueHandle`, and `Change` support `Symbol.dispose`.

In contrast, `Snapshot` and `Update` are pure, immutable JavaScript objects
holding pre-encoded binary envelopes (`.bytes`). They hold no WebAssembly handles
and never need manual `dispose()`.

Next: [Events and lifecycle](/docs/document/events-lifecycle).
