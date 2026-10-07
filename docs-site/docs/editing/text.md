# Text and RichText editing

`tx.text(path)` and `tx.richText(path)` open editors for a [Text or RichText](/docs/core/text)
value. Each call is interpreted against the transaction's working content, so later
calls see the effect of earlier ones.

## Edit Text

```ts
import { Document, text } from 'colla-ot'
const doc = Document.create({ title: text('A😀B') })
doc.edit(tx => {
  const title = tx.text(['title'])
  title.insert(3, '!')
  console.log('Working text after insertion:', tx.get(['title'])?.toJS()) // Text containing A😀!B
  title.replace(1, 2, '🐬')
  console.log('Working text after replacement:', tx.get(['title'])?.toJS()) // Text containing A🐬!B
  title.delete(3, 1)
})
console.log('Committed text:', doc.get(['title'])?.toJS()) // Text containing A🐬B
doc.close()
```

`insert(index, text)`, `delete(index, count)` and `replace(index, count, text)` take
UTF-16 offsets and counts, the same units as browser selections. In `A😀B`, index 3 is
after the emoji; index 2 splits its surrogate pair and fails with
`invalid_utf16_boundary`. The resulting Change records Unicode scalar positions; see
[Positions](/docs/core/positions).

## Edit RichText

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

| Method                              | Effect                                         |
| ----------------------------------- | ---------------------------------------------- |
| `insertText(index, text, attrs?)`   | Inserts attributed text                        |
| `insertEmbed(index, value, attrs?)` | Inserts an atomic embed of length one          |
| `delete(index, count)`              | Removes text and embeds                        |
| `replace(index, count, spans)`      | Removes a range and inserts spans in its place |
| `format(index, count, patch)`       | Sets attributes; `null` removes an attribute   |

Offsets are UTF-16 code units, with each embed counting as one position.

## Failures

An offset past the end, an offset inside a surrogate pair, an unsupported attribute
value or a target of the wrong kind throws and abandons the entire transaction,
including edits made earlier in the same callback. Editors are valid only inside their
callback; see [Transactions and editors](./transactions#scope-restrictions).
