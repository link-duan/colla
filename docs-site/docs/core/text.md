# Text and RichText

Text and RichText are sequences that collaborators edit character by character.
Concurrent insertions and deletions in the same field merge instead of replacing each
other. A plain string is an atomic [String](./values#choose-the-right-kind) and cannot
be edited this way.

## Text

`text(value)` creates collaborative plain text. Text must be valid Unicode: a string with
an unpaired UTF-16 surrogate is rejected with `invalid_value`. Its projection is an
immutable `Text` wrapper with `type: 'text'` and the string in `value`.

```ts
import { Value, text } from 'colla-ot'

const title = Value.fromJS(text('A😀B'))
console.log('Projection:', title.toJS()) // Text { type: 'text', value: 'A😀B' }
console.log('Same as String:', title.equals(Value.fromJS('A😀B'))) // false
```

Text and String with the same characters are different Values.

## RichText

`richText(spans)` creates a sequence of attributed text and atomic embeds:

| Span  | Shape                              | Length                    |
| ----- | ---------------------------------- | ------------------------- |
| Text  | `{ type: 'text', text, attrs? }`   | Its characters            |
| Embed | `{ type: 'embed', value, attrs? }` | 1, whatever `value` holds |

Attribute values are atomic: boolean, bigint, finite number or string. An embed's
`value` is any Value, such as `{ image: 'asset-123' }`. It is atomic within the
sequence: edits replace or delete the embed as a whole, and there is no Path into it.

```ts
import { Value, richText } from 'colla-ot'

const body = Value.fromJS(
  richText([
    { type: 'text', text: 'Hel', attrs: { bold: true } },
    { type: 'text', text: 'lo', attrs: { bold: true } },
    { type: 'embed', value: { image: 'asset-123' }, attrs: { alt: 'Diagram' } },
  ]),
)
console.log('Spans:', body.toJS()) // one bold 'Hello' span, then the embed
```

RichText is stored in a normalized form: empty text spans are removed and adjacent text
spans with equal attributes merge. Base application behavior on content and attributes,
never on where span boundaries fall.

## Formatting

Formatting is an attribute patch applied to a range. A key set to a value adds or
replaces that attribute; a key set to `null` removes it. `null` only appears in patches,
never as a stored attribute value. Concurrent text edits and formatting on the same
RichText transform together in one sequence algebra. See [Changes](./changes#text-and-richtext-operations) for the
operation format and [Text and RichText editing](/docs/editing/text) for editor methods.

## What Colla does not decide

RichText is not HTML and does not prescribe an editor schema. Colla validates the model,
but your application chooses which attributes and embed values are meaningful, how they
render, and how URLs and other rendered content are sanitized.
