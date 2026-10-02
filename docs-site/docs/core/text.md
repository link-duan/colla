# Text

Use `text('Draft')` for collaboratively editable text. A plain string is an atomic
value and cannot be opened with `tx.text`. Text is valid Unicode; malformed strings
with unpaired UTF-16 surrogates are rejected.

## High-level editing

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

JavaScript editor offsets and deletion counts are UTF-16 code units, evaluated against
the working content at each step. In `A😀B`, index 3 is after the emoji; index 2 splits
its surrogate pair and raises `invalid_utf16_boundary`. Out-of-range spans also fail.
A failed operation rolls back the entire transaction.

## Low-level changes

Change Text operations use ordered retain, insert and delete streams with Unicode
scalar lengths. The same emoji has length one in that interface. Retained trailing
content need not be written explicitly. Rust text editing also uses scalar coordinates.
For example, retaining `A😀` requires scalar length 2, while a high-level JavaScript
editor inserts after it at UTF-16 offset 3.

For editor lifetime, see [Runtime lifecycle](/docs/editing/lifecycle#transaction-and-event-boundaries).
Read [Text coordinates](./coordinates) before translating low-level Edit Steps to UI offsets.
