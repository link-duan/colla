---
title: RichText
description: Attributed text and embeds in Colla Core.
---

# RichText

<p class="lead">RichText is a normalized sequence of attributed text spans and atomic embedded Values.</p>

## Shape

Construct RichText with `richText(spans)`. A text span has `type: 'text'`, a
string, and optional attributes. An embed has `type: 'embed'`, a Value, and
optional attributes:

```ts
import { richText } from 'colla-ot/core'

const body = richText([
  { type: 'text', text: 'Hello ', attrs: { bold: true } },
  { type: 'embed', value: { id: 'mention-1' }, attrs: { kind: 'mention' } },
  { type: 'text', text: '!', attrs: { bold: true } },
])
```

Each embed occupies one logical sequence unit regardless of the size of its
embedded Value. The embed itself is atomic to RichText operations: it can be
inserted, deleted, retained, or formatted, but cannot be recursively modified
inside the sequence. If embedded state needs an independent collaborative
lifecycle, keep it at a stable Value path and reference it from the embed.

## Normalization

Construction freezes the result and applies canonical normalization:

- empty text spans are removed;
- adjacent text spans with identical attributes are merged;
- attribute keys are unique and ordered canonically;
- attribute values are booleans, signed 64-bit integers, finite numbers, or strings;
- unpaired UTF-16 surrogates and unsupported values are rejected.

Normalization affects representation, not meaning. Cached span indexes and text
lengths are implementation details and do not participate in equality or the
binary representation.

## RichText changes

The builder supports retaining a range, inserting text or one embed, and
deleting a range. A retain may carry an attribute patch:

```ts
import { Change } from 'colla-ot/core'

const format = Change.build(change => {
  change.richText(richText => {
    richText.retain(5, attrs => {
      attrs.set('bold', true)
      attrs.remove('color')
    })
    richText.insertText('!', { italic: true })
  })
})
```

Patches use explicit `set` and `remove` actions. `null` is not a deletion
sentinel, and attribute values cannot be arrays, objects, or RichText values.
As with Text, lengths count Unicode scalars plus one unit per embed. Adjacent
compatible operations are merged and zero-length operations disappear.

Apply RichText changes with the regular Core algebra (`apply`, `compose`,
`invert`, and `transformPair`). For editor-facing paths and ranges, read
[Coordinates](/docs/core/coordinates); for the complete API, see the
[JavaScript reference](/reference/javascript).

Next: [Coordinates](/docs/core/coordinates).
