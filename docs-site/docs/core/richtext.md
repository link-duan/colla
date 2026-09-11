# RichText

RichText is a sequence of attributed text spans and atomic embeds. It is not HTML and
does not prescribe an editor's schema. Adjacent compatible spans can be normalized,
so application behavior should depend on content and attributes, not span boundaries.

## Insert and format

```ts
import { Document, richText } from 'colla-ot'
const doc = Document.create({ body: richText([]) })
doc.edit(tx => {
  const body = tx.richText(['body'])
  body.insertText(0, 'Hello', { bold: true })
  body.insertEmbed(5, { image: 'asset-123' }, { alt: 'Diagram' })
  body.format(0, 5, { bold: null, italic: true })
})
console.log('Formatted content:', doc.get(['body'])?.toJS())
doc.close()
```

A text span has `{ type: 'text', text, attrs? }`; an embed has
`{ type: 'embed', value, attrs? }`. Attributes hold boolean, bigint, finite number or
string values. A format patch uses null to remove an attribute; null is not a stored
attribute value. Concurrent text and formatting participate in OT together.

## Positions and atomic embeds

High-level JavaScript positions count UTF-16 code units, with each embed occupying one
position. Low-level RichText changes count Unicode scalars, also counting each embed as
one. Embeds carry a Core Value but are atomic in the surrounding rich-text sequence;
they are not independently editable nested documents through sequence paths.

## Integration boundaries

`replace(index, count, spans)` combines replacement with span insertion; `delete`
removes a range. Invalid ranges or unsupported attributes abort the transaction.
Sanitize URLs and rendered content in the application. Colla validates model input,
but it does not make HTML safe or choose how an editor displays an embed.

See [Editor integration](/docs/editing/editor-integration) for event handling.
