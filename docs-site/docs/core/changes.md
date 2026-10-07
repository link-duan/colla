# Changes

A Change is an immutable, ordered sequence of path-addressed operations. It describes
an edit only: it carries neither the content it applies to nor a server revision. Most
applications produce Changes through [Transactions](/docs/editing/transactions) and
receive them in edit results; construct them directly when replaying, testing or
processing edits without a Document.

## Operations

| `type`     | Fields               | Effect and precondition                                                        |
| ---------- | -------------------- | ------------------------------------------------------------------------------ |
| `insert`   | `path`, `value`      | Inserts `value` into a List. The last segment is an index ≤ length             |
| `delete`   | `path`               | Removes an existing Map member or List item                                    |
| `set`      | `path`, `value`      | Writes a Map member, adding it if missing; replaces a List item or the root    |
| `listMove` | `path`, `from`, `to` | Moves one item within the List at `path`; `to` is read after removing the item |
| `add`      | `path`, `delta`      | Adds a bigint to an existing Int; overflow outside i64 fails                   |
| `text`     | `path`, `operations` | Edits the Text at `path` with a sequence of steps                              |
| `richtext` | `path`, `operations` | Edits and formats the RichText at `path` with a sequence of steps              |

`value` is a [Value](./values). Map members are written only with `set`; an `insert`
whose Path ends with a Map key is rejected. A `set` on a List index replaces an existing
item and never appends. The root can be replaced with `set` but cannot be the target of
`insert` or `delete`.

## Operations run in order

Each operation's Path and positions are interpreted against the content produced by the
operations before it, exactly as if they were applied one by one:

```ts
import { Change, Value, apply, text } from 'colla-ot'

const base = Value.fromJS({ tasks: ['draft', 'review'] })
const change = Change.create([
  { type: 'insert', path: ['tasks', 0], value: Value.fromJS('plan') },
  // 'review' moved from index 1 to index 2 because of the insertion above.
  { type: 'set', path: ['tasks', 2], value: Value.fromJS('approve') },
  { type: 'listMove', path: ['tasks'], from: 2, to: 0 },
  { type: 'set', path: ['note'], value: Value.fromJS(text('Ready')) }, // adds a Map member
])
console.log('Result:', apply(base, change).toJS())
// { note: Text { type: 'text', value: 'Ready' }, tasks: [ 'approve', 'plan', 'draft' ] }
```

## Text and RichText operations

A `text` operation holds steps that walk the sequence from position 0:

| Step                         | Effect                               |
| ---------------------------- | ------------------------------------ |
| `{ type: 'retain', length }` | Skips characters                     |
| `{ type: 'insert', text }`   | Inserts text at the current position |
| `{ type: 'delete', length }` | Removes characters                   |

A `richtext` operation uses the same three steps, except that insert carries a
[span](./text#richtext) (`{ type: 'insert', span }`) and retain may carry an attribute
patch (`{ type: 'retain', length, attrs }`) that formats the retained range. Lengths are
Unicode scalars, with each embed counting as one; see [Positions](./positions). Content
after the last step is retained implicitly. Steps that run past the end fail with
`out_of_bounds`.

```ts
import { Change, Value, apply, text } from 'colla-ot'

const base = Value.fromJS(text('A😀B'))
const change = Change.create([
  {
    type: 'text',
    path: [],
    operations: [
      { type: 'retain', length: 2 }, // 'A' and the emoji: two scalars
      { type: 'insert', text: '!' },
    ],
  },
])
console.log('Result:', apply(base, change).toJS()) // Text { type: 'text', value: 'A😀!B' }
```

## Construction and canonical form

`Change.create(operations)` validates each operation's shape and stores it in canonical
form: it drops operations that are no-ops on any content (a `listMove` with
`from === to`, an `add` of zero, an empty step list), merges adjacent steps of the same
kind, and normalizes RichText spans and patches. `operations` returns the canonical
sequence. `Change.noop()` and `isNoop` represent and detect the empty Change.

`Change.create` does not check operations against any content; a missing target or an
out-of-range index fails when the Change is applied. Canonical form describes one
Change; two different operation sequences with the same effect are not made equal.

## Persist and exchange

`encode()` returns canonical bytes and `Change.decode(bytes)` restores the Change.
Decoding rejects any non-canonical or malformed input with `invalid_encoding`, so a
decoded Change is as valid as a constructed one. A Change holds at most 1,000,000
operations. Store the base content, or a revision that identifies it, alongside a
Change; see [Change algebra](./algebra) for why every operation needs one.
