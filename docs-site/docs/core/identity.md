# Element identity and paths

An ElementId names an element instance. A Path describes where an element is in one
snapshot. List insertions and Moves can change a Path while the ElementId stays stable.
Use IDs for selections or references that should follow a moved element.

## Locate an element

```ts
import { Document, text } from 'colla-ot'
const doc = Document.create({ tasks: [{ title: text('Draft') }], done: [] })
const task = doc.idAt(['tasks', 0])
doc.edit(tx => tx.move(task, { parent: ['done'], index: 0 }))
console.log('New path, same task ID:', doc.pathOf(task)) // ['done', 0]
console.log('Lookup by original ID:', doc.get(task)?.kind()) // 'map'
doc.close()
```

A Path is an array of string Map keys and numeric List indexes; `[]` names the root.
Characters inside Text/RichText are addressed by sequence coordinates, not by element IDs.
A Ref does not cause a Path to follow its target automatically.

## Read failure and snapshot boundaries

`get`, `kind`, `pathOf` and `resolve` return undefined for absent results. `has` returns
false; `idAt` requires an existing target and throws if none exists. Invalid argument
shapes still fail, even for read operations. An ID from another document is not a
cross-document lookup mechanism.

Keep the snapshot used to calculate a Path when you need a consistent read. An older
Value continues to report the old location after the Document changes. Never persist
Paths as a substitute for the identities preserved by the binary codec.

Continue with [References](./references) and [Coordinates](./coordinates).
