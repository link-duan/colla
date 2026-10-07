# Positions

A position is a gap between characters in Text or RichText. Colla counts positions in
two units, depending on the API. Mixing them up shifts edits in any text containing
characters outside the Basic Multilingual Plane, such as emoji.

## Two units

| Unit              | Used by                                                       |
| ----------------- | ------------------------------------------------------------- |
| UTF-16 code units | JavaScript `TextEditor` and `RichTextEditor` methods          |
| Unicode scalars   | Change operations, `EditResult.change`, all Rust text editing |

The JavaScript editors accept the same offsets as browser selections and `String`
methods. Changes use Unicode scalars so that they mean the same thing in every language.
In both units, a RichText embed occupies one position.

For `A😀B`:

| Boundary       | Before A | After A | After 😀 | After B |
| -------------- | -------- | ------- | -------- | ------- |
| UTF-16         | 0        | 1       | 3        | 4       |
| Unicode scalar | 0        | 1       | 2        | 3       |

UTF-16 offset 2 falls inside the emoji's surrogate pair and is not a valid position;
the editors reject it with `invalid_utf16_boundary`. Neither unit counts user-perceived
characters: a flag or an accented letter built from combining marks occupies several
positions in both. Grapheme-aware cursor movement belongs to your editor.

## Convert between units

Convert against the content the position refers to:

```ts
import { text } from 'colla-ot'

function scalarToUtf16(content: string, scalar: number): number {
  let offset = 0
  for (const char of content) {
    if (scalar-- === 0) break
    offset += char.length
  }
  return offset
}
function utf16ToScalar(content: string, offset: number): number {
  let scalar = 0
  for (const char of content) {
    if (offset <= 0) break
    offset -= char.length
    scalar++
  }
  if (offset < 0) throw new RangeError('offset splits a surrogate pair')
  return scalar
}

const content = text('A😀B').value
console.log('Scalar 2 as UTF-16:', scalarToUtf16(content, 2)) // 3
console.log('UTF-16 3 as scalar:', utf16ToScalar(content, 3)) // 2
```

For RichText, walk the spans and count each embed as one position in both units.

## Convert step by step

Every position in a Change is relative to the content left by the preceding operations,
and the same holds for successive editor calls in one transaction. When translating a
Change into UI offsets, apply each operation to your mirror of the content and convert
the next operation's positions against the updated content, not against the original
snapshot. The [editor adapter example](/docs/examples/editor) follows this pattern, and
[Editor integration](/docs/editing/editor-integration) describes the full adapter.
