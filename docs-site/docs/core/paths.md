# Paths

A Path describes where an element is in one content state. It is an array of string
Map keys and numeric List indexes; `[]` names the root. Values carry no identity, so
a Path is the only way to address content.

## Locate an element

```ts
import { Document, text } from 'colla-ot'
const doc = Document.create({ tasks: [{ title: text('Draft') }, { title: text('Review') }] })
doc.edit(tx => tx.list(['tasks']).move(0, 1))
console.log('Moved title:', doc.get(['tasks', 1, 'title'])?.toJS()) // Text containing 'Draft'
doc.close()
```

Characters inside Text/RichText are addressed by sequence coordinates, not by Paths.

## Read failure and snapshot boundaries

`get` and `kind` return undefined, and `has` returns false, for `missing_key` or
`out_of_bounds`. They still throw `type_mismatch` if a Path traverses a scalar or uses
a segment incompatible with the current container. For example, with `{ n: 0n }`,
`get(['missing'])` returns undefined, but `get(['n', 'child'])` throws; `kind` and
`has` follow the same distinction. Invalid arguments and lifecycle errors still fail,
even for read operations.

## Paths change with edits

A Path is interpreted against the content it was computed from. An insertion, deletion
or ListMove before an element changes that element's Path. Keep the snapshot used to
calculate a Path when you need a consistent read; an older Value continues to show the
old content after the Document changes.

Within a transaction, each editing call interprets its Path against the current
working content. Collaborative transformation rewrites the Paths of concurrent
operations, so remote edits land on the intended element. Applications that keep Paths
outside the library, such as an editor selection, must update them after each edit.
When an element needs a stable application key, store it in the content, for example
as an `id` field in a Map.

Continue with [Coordinates](./coordinates).
