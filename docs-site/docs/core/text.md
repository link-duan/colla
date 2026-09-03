---
title: Text
description: Unicode-aware collaborative text in Colla Core.
---

# Text

<p class="lead">Text is an explicit collaborative sequence. Its retain and delete lengths count Unicode scalar values, so Rust and JavaScript agree on positions.</p>

## Text values

Use `text(value)` to distinguish collaborative text from an atomic `string`:

```ts
import { text, ValueHandle } from 'colla-ot/core'

const value = ValueHandle.fromJS(text('A😀B'))
```

The constructor requires a string and rejects unpaired UTF-16 surrogates. The
returned `{ type: 'text', value }` record is immutable. Text is still stored as
valid UTF-8; JavaScript UTF-16 is only a host representation.

## Text changes

Text changes are left-to-right streams of `retain(length)`, `insert(text)`, and
`delete(length)` operations. Use `Change.build` for readable construction:

```ts
import { Change, apply } from 'colla-ot/core'

const change = Change.build(change => {
  change.text(text => text.retain(1).delete(1).insert('😀'))
})
const next = apply(value, change)
```

For `A😀B`, the sequence length is 3: `A`, `😀`, `B`. The emoji counts as one
Unicode scalar, even though JavaScript reports two UTF-16 code units in
`'A😀B'.length`. A retain or delete that splits a scalar is invalid. An omitted
tail is retained implicitly, and constructors merge adjacent compatible ops and
remove zero-length operations.

## Applying and inspecting edits

`apply()` returns a new `ValueHandle`; it does not alter the base. To drive an
editor, `convertChangeToEditSteps(change, base)` projects the recursive Change
to path-relative text operations. `inspectChange(change, base)` returns a
human-readable `ChangeView`, including text insertion positions and deletion
ranges.

```ts
import {
  Change, ValueHandle, apply, convertChangeToEditSteps, text,
} from 'colla-ot/core'

const base = ValueHandle.fromJS(text('Draft'))
const edit = Change.build(change => change.text(text => text.retain(5).insert(' v2')))
const steps = convertChangeToEditSteps(edit, base)
// [{ type: 'text', path: [], ops: [...]}]
const result = apply(base, edit)
```

The base is required for projection because a Change stores an operation, not
the original content. Paths and editor offsets are views of that snapshot and
are not part of the Change wire format. See [Coordinates](/docs/core/coordinates)
for conversion to and from UTF-16 offsets.

## Limits and errors

`Change.fromJS` and `Change.build` accept `InputOptions` to cap sequence length,
operation count, string bytes, and recursive depth. Invalid types, out-of-range
lengths, malformed strings, and oversized input raise `CollaError` with a
stable error code. Use the [JavaScript API reference](/reference/javascript)
for the complete builder and error surface.

Next: [RichText](/docs/core/richtext).
