---
title: Coordinates
description: Snapshot-relative paths and Unicode-aware positions in Colla Core.
---

# Coordinates

<p class="lead">Coordinates are temporary views into a particular Value. They connect Core’s Unicode-scalar model to editor APIs without becoming part of the Change format.</p>

## Paths

The JavaScript `Path` type is a readonly array of string map keys and
non-negative safe-integer list indexes. The root is `[]`:

```ts
import { ValueHandle, text } from 'colla-ot/core'

const value = ValueHandle.fromJS({
  sections: [{ body: text('A short paragraph') }],
})
const bodyPath = ['sections', 0, 'body']
value.kind(bodyPath)
value.get(bodyPath)
```

Paths are snapshot-relative. A list insertion or deletion can shift every later
index, and a concurrent map or sequence edit can make a path invalid. Therefore
paths are useful for lookup, diagnostics, and editor projections, but are not
stable object identifiers and are not encoded inside a Change.

`ValueHandle.kind(path)`, `has(path)`, and `get(path)` validate each segment and
report failures such as `missing_key`, `out_of_bounds`, or `type_mismatch` as a
classified `CollaError`.

## Unicode positions

Core sequence lengths count Unicode scalar values. JavaScript editor APIs often
use UTF-16 offsets, so the Core package provides explicit conversion functions:

```ts
import {
  resolveCodePointPosition,
  resolveUtf16Position,
  text,
  ValueHandle,
} from 'colla-ot/core'

const value = ValueHandle.fromJS({ body: text('A😀B') })

resolveCodePointPosition(value, ['body'], 3) // 2
resolveUtf16Position(value, ['body'], 2)     // 3
```

`resolveCodePointPosition(value, path, utf16Position)` converts a UTF-16 offset
to a Unicode-scalar offset. `resolveUtf16Position(value, path,
codePointPosition)` performs the reverse conversion. Both validate the path and
range against the supplied snapshot. An offset inside an emoji’s surrogate
pair raises `invalid_utf16_boundary` rather than silently splitting it.

RichText text uses scalar positions, while an embed counts as one sequence unit.
This makes the same retain/delete lengths meaningful in Rust and JavaScript.

## Coordinates and edit steps

`convertChangeToEditSteps(change, base)` produces recursive steps with a path
and sequence operations. Use the base Value when converting because the same
Change can only be interpreted relative to its expected snapshot. Convert
editor UTF-16 selections at the boundary, then build Core Changes using scalar
lengths.

Coordinates do not carry revisions or survive rebasing. The `Document` layer
emits edit steps after applying and rebasing updates; see the
[document lifecycle](/docs/document/lifecycle) and the
[OT model](/docs/ot/).

Next: [Values](/docs/core/values).
