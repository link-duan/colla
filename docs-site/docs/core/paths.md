# Paths

A Path is the only way to address content. It is an array of segments: a string selects
a Map key and a number selects a List index. `[]` is the root. Characters inside Text and
RichText are not Path segments; they are addressed by [Positions](./positions).

## Read with a Path

```ts
import { Value, text } from 'colla-ot'

const value = Value.fromJS({ tasks: [{ title: text('Draft'), done: false }], count: 1n })
console.log('Title kind:', value.kind(['tasks', 0, 'title'])) // text
console.log('Done:', value.get(['tasks', 0, 'done'])?.toJS()) // false
console.log('Second task exists:', value.has(['tasks', 1])) // false
```

`get`, `has` and `kind` are available on Values, Documents and Transactions, with the
same rules:

| Path                                         | Result                           |
| -------------------------------------------- | -------------------------------- |
| Names an element                             | The element                      |
| Missing Map key or List index past the end   | `undefined`; `has` returns false |
| Steps into a scalar, or uses a key on a List | Throws `type_mismatch`           |

A missing element is an ordinary answer; a Path whose shape contradicts the content is
a bug. For example, with `{ count: 1n }`, `get(['missing'])` returns undefined, but
`get(['count', 'child'])` throws.

## A Path belongs to one content state

A Path is interpreted against the content it was computed from: a snapshot, the working
content of a transaction, or the content produced by the preceding operations of a
Change. Inserting, deleting or moving an earlier List item changes the index of every
later item, so the same Path can name a different element after an edit.

- **Within a Change or a transaction**, each operation's Path is interpreted against
  the content left by the operations before it. See [Changes](./changes).
- **Across concurrent edits**, transformation rewrites the Paths inside the other
  Change, so remote edits land on the element they were made against. See
  [Concurrent edits](./concurrency).
- **Outside the library**, such as a selection held by an editor, your application
  must update stored Paths after each edit, or resolve them again from content.

To refer to an item durably, store a stable key in its Map and search for it when
needed. To read consistently while edits continue, keep the immutable snapshot that the
Path was computed from; older Values never change.
