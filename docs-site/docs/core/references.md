# References

A Ref is an atomic weak reference to an ElementId in the same document. It adds a
relationship without adding ownership. Ref graphs may contain cycles; the owning
Map/List structure must remain a tree.

## Create and resolve

```ts
import { Document, ref } from 'colla-ot'
const doc = Document.create({ item: { label: 'Draft' }, selected: null })
const item = doc.idAt(['item'])
doc.edit(tx => tx.set(['selected'], ref(item)))
console.log('Ref resolves to the item:', doc.resolve(ref(item))?.id === item) // true
console.log('Referring elements:', doc.referencesTo(item)) // IDs of referring elements
const snapshot = doc.snapshot()
doc.edit(tx => tx.delete(item))
console.log('After deletion — current document:', doc.resolve(ref(item))) // undefined
console.log('Older snapshot still resolves the item:', snapshot.resolve(ref(item))?.id === item) // true
doc.close()
```

Resolution advances exactly one hop in the queried snapshot. If the target itself
contains a Ref, resolving further is an explicit application action. Ordinary `get`
and Path traversal never follow Refs implicitly.

## Deletion and copies

Deleting a target leaves the stored Ref intact and dangling. A later undo that restores
the same target ID makes it resolvable again. Copy remaps references whose targets are
inside the copied subtree; references to outside elements retain their original targets.

Refs do not keep deleted targets alive and do not grant access to another document.
`referencesTo(id)` includes references inside atomic RichText embeds. The reverse index
is derived from a snapshot and does not become part of its canonical bytes.

See [Move and Ref example](/docs/examples/move-ref) to follow a reference across a move.
