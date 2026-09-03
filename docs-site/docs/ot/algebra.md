---
title: Apply, Compose, and Invert
description: Sequential Colla OT operations over immutable Values and Changes.
---

# Apply, Compose, and Invert

<p class="lead">Use these operations for deterministic transitions, batching, and undo without mutating the original Value or Change.</p>

## Apply a Change

`apply(base, change)` evaluates a canonical Change against one concrete Value
and returns a new Value. It validates the recursive type at every path, Map key
presence, List/Text/RichText ranges, and checked integer arithmetic.

```ts
import { Change, ValueHandle, apply, text } from 'colla-ot/core'

const before = ValueHandle.fromJS(text('Draft'))
const edit = Change.build(change => {
  change.text(textEdit => textEdit.retain(5).insert(' v2'))
})

const after = apply(before, edit)
// after.toJS() -> { type: 'text', value: 'Draft v2' }
```

The base remains usable after success or failure. A failed operation does not
commit a partial nested result. Match JavaScript failures by
`CollaError.code`—for example `type_mismatch`, `out_of_bounds`, or
`integer_overflow`—rather than by human-readable message text.

## Compose sequential Changes

`compose(first, second)` combines two Changes where `second` is written against
the value produced by `first`. The result is one canonical Change with the same
observable effect:

```text
apply(apply(v, first), second) == apply(v, compose(first, second))
```

```ts
import { Change, ValueHandle, apply, compose, text } from 'colla-ot/core'

const base = ValueHandle.fromJS(text('ab'))
const first = Change.build(change => change.text(t => t.retain(1).insert('x')))
const second = Change.build(change => change.text(t => t.retain(2).insert('y')))
const combined = compose(first, second)

apply(base, combined).toJS() // { type: 'text', value: 'axyb' }
```

Composition recurses through Map modifications and consumes sequence streams
without expanding a logical retain or delete into one operation per element.
It can fail when root kinds or sequential Map entries are incompatible, or
when a second sequence cannot consume the first sequence's output.

## Build an inverse

`invert(change, base)` produces a Change that restores the original Value after
the forward Change:

```text
apply(apply(base, change), invert(change, base)) == base
```

The base is mandatory because a Change intentionally carries no old values. A
delete inverse must reinsert the deleted content; a replacement inverse must
know the previous type and value. Keep the exact pre-change Value when creating
an undo record, not a later snapshot that happens to have the same shape.

```rust
use colla::{apply, invert, Change, TextChange, TextOp, Value};

let base = Value::text("ab");
let change: Change = TextChange::from_ops([
    TextOp::Retain(1),
    TextOp::Delete(1),
])?.into();
let after = apply(&base, &change)?;
let undo = invert(&change, &base)?;
assert_eq!(apply(&after, &undo)?, base);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Integer addition uses checked arithmetic. If negating an extreme delta is not
representable, the inverse can use a replacement with the original base so the
round-trip identity still holds.

## Canonical results and ownership

Public constructors and algebra results normalize equivalent operation streams:
empty operations disappear, compatible adjacent operations merge, and trailing
plain retains are omitted. A typed empty Change and `IntChange::Add(0)` become
`Noop`. Raw Core bytes remain a strict structural format; see the
[protocol boundaries](./protocol-boundaries).

Rust returns owned immutable Values/Changes. JavaScript returns independent
handles; algebra does not consume its inputs. Dispose long-lived handles when
you control allocation peaks, and treat editor `EditStep` projections as views,
not as Change input or wire data.

Next: [Transform and rebasing](./transform-rebase), then
[Concurrency](./concurrency).

