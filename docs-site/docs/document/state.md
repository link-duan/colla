---
title: Document state
description: Visible state, confirmed state, revisions, and pending local work.
---

# Document state

`Document` maintains two Core values and two revision counters internally:

| State | Meaning |
| --- | --- |
| Visible value | What the editor should display, including pending local edits. |
| Confirmed value | The value represented by the latest acknowledged server revision. |
| Visible revision | Starts at the Snapshot revision and advances for local and remote transitions. |
| Confirmed revision | Starts at the Snapshot revision and advances only when remote work is accepted or a local Update is acknowledged. |

The pending queue stores local Changes in creation order. Each pending Update
has an instance-local `updateId`; after rebasing, its revision is rewritten to
the next confirmed revision while its ID remains stable.

```text
confirmed value @ r ── local change ──> visible value @ r+1
       │                                      │
       └── remote update @ r ── rebase ───────┘
                                      │
                          ack oldest pending ID
                                      │
                         confirmed value @ r+1
```

## Observable operations

```ts
const document = Document.fromJS({ title: 'Draft', count: 0n }, 10n)

document.revision       // 10n
document.get(['title']) // direct Value access without creating a handle
document.has(['title']) // true
document.kind(['title'])// 'string'
document.value()        // independently owned visible ValueHandle
document.snapshot()     // Snapshot of visible value and revision
```

`get()`, `has()`, and `kind()` inspect visible content directly without needing
handle lifecycle management. `value()` returns a clone, so disposing or
inspecting it cannot mutate the Document. `snapshot()` creates a durable
content checkpoint without changing state. A disposed Document rejects later
operations with `invalid_state`.

## Invariants and failures

- Visible content contains confirmed content plus every pending local Change.
- Remote Updates must name exactly the current confirmed revision.
- Acknowledgements remove only the oldest pending Update.
- Failed apply, transform, revision, or acknowledgement operations leave the
  existing state and pending queue usable.
- `revision` and `updateId` are unsigned 64-bit `bigint` values.

The application should keep retry metadata and durable queue records outside the
Document. See [lifecycle](./lifecycle) for transitions and [Snapshot and
Update](./snapshot-update) for checkpoint boundaries.

Next: [Local and remote updates](/docs/document/local-remote).
