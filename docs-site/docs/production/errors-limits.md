---
title: Errors and input limits
description: Bound untrusted input and handle Colla failures without hiding state transitions.
---

# Errors and input limits

Treat every JavaScript value and every byte received from another process as
untrusted. Validate at the boundary, branch on stable error codes, and keep the
last known-good `Document` state when an operation is rejected.

## Which boundary owns which limit?

| Input boundary | Colla behavior | Application policy |
| --- | --- | --- |
| Structured Value or Change input | `InputOptions.limits` bounds depth, nodes, containers, strings, and sequence work. | Choose limits for the request and tenant before calling `fromJS` or `build`. |
| Core Value or Change bytes | The canonical codec checks tags, lengths, UTF-8, recursion, and trailing bytes. | Reject oversized request bodies before decoding and enforce endpoint quotas. |
| Snapshot or Update bytes | The envelope checks magic, protocol version, tuple shape, and complete input. | Select the expected document, authenticate the sender, and enforce replay policy. |
| Remote Update application | `Document` checks the next confirmed revision and Change compatibility. | Replay a gap or request a Snapshot; never force an unrelated Update. |

`InputOptions` applies to structured JavaScript construction:
`ValueHandle.fromJS`, `Change.fromJS`, and `Change.build`. It does not add a
per-call payload-size option to `ValueHandle.decode`, `Change.decode`,
`Snapshot.decode`, or `Update.decode`; enforce byte-size and endpoint limits in
the application before decoding.

## Configure structured-input limits

```ts
import { Change, ValueHandle } from 'colla-ot/core'

const limits = {
  maxDepth: 32,
  maxValueNodes: 50_000,
  maxChangeNodes: 50_000,
  maxContainerLength: 10_000,
  maxStringBytes: 1_000_000,
  maxSequenceOps: 10_000,
  maxSequenceLength: 100_000,
}

const value = ValueHandle.fromJS(request.body.value, { limits })
const change = Change.fromJS(request.body.change, { limits })
```

Limits are counted against raw input before semantic normalization. Empty
operations therefore cannot bypass a resource policy. Keep separate budgets for
interactive edits, imports, and administrative repair jobs when their workloads
differ. The [JavaScript reference](/reference/javascript) lists every default.

## Handle failures by code

```ts
import { Document, Update } from 'colla-ot'
import { CollaError } from 'colla-ot/core'

try {
  const update = Update.decode(bytes)
  document.applyRemote(update)
} catch (error) {
  if (error instanceof CollaError) {
    metrics.increment(`colla.${error.code}`)
    audit.record({ operation: error.operation, code: error.code })
    if (error.code === 'incompatible_change') recovery.requestSnapshot()
  }
  throw error
}
```

Common classifications include `invalid_encoding`, `limit_exceeded`,
`type_mismatch`, `missing_key`, `key_already_exists`, `out_of_bounds`,
`integer_overflow`, `incompatible_change`, `invalid_value`, `invalid_state`,
and `invalid_argument`. Match `code`, `operation`, optional `path`, and
structured `details`; human-readable messages are diagnostics, not protocol
fields.

## Recovery rules

- A failed local or remote operation is atomic; keep the existing value,
  revision, and pending queue.
- Rate-limit repeated malformed requests and avoid logging raw document bytes.
- Include a tenant- and request-scoped identifier in telemetry, but do not use a
  local `updateId` as a global deduplication key.
- After an unrecoverable revision gap, replay from the confirmed revision or
  restore a trusted Snapshot with an application-owned queue.
- Never write placeholder content to make a rejected Change look successful.

Next: [Synchronization protocol](./sync-protocol), then
[Production testing](./testing).
