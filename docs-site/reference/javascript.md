---
title: JavaScript API reference
description: Import paths, public types, and runtime contracts for colla-ot.
---

# JavaScript API

<p class="eyebrow">Reference</p>
<p class="lead">One WebAssembly-backed package exposes document state, immutable values and changes, codecs, and OT operations from a single JavaScript entry point: <code>colla-ot</code>.</p>

This page is an index of the stable public surface. For task-oriented examples,
start with [Getting started](/docs/getting-started), then read the
[Document state](/docs/document/) or the [JavaScript example](/docs/examples/javascript-document).
The generated declaration files shipped in the npm package are the definitive
TypeScript signatures.

## Import

All public JavaScript symbols are imported from `colla-ot`:

```ts
import {
  Document,
  Snapshot,
  Update,
  ValueHandle,
  Change,
  apply,
  compose,
  invert,
  transformPair,
  text,
} from 'colla-ot'
```


## Document

### `Document`

`Document` owns the current visible `ValueHandle`, a confirmed revision, and
the queue of local Updates that have not been acknowledged. It applies local
changes optimistically and rebases those pending changes when an ordered remote
Update arrives.

| Member | Contract |
| --- | --- |
| `Document.fromJS(value, revision?, options?)` | Create a document from structured Core Value input; revision accepts `number` or `bigint` (defaults to `0n`). |
| `Document.fromSnapshot(snapshot)` | Restore visible content and revision from a `Snapshot`; pending state is empty. |
| `revision` | Current visible revision as an unsigned 64-bit `bigint`. |
| `confirmedRevision` | Latest confirmed revision acknowledged by server as an unsigned 64-bit `bigint`. |
| `hasPending` | Boolean indicating whether there are unconfirmed pending local changes. |
| `pendingCount` | Number of unconfirmed local updates currently in the pending queue. |
| `value()` | Return an independently owned `ValueHandle` for visible content. |
| `get(path)` | Resolve a Snapshot-relative Path and borrow target `Value` without handle management. |
| `has(path)` | Check if a Snapshot-relative Path exists in visible content. |
| `kind(path?)` | Return the `ValueKind` at the given path (defaults to root). |
| `resolveCodePointPosition(path, pos)` | Convert a UTF-16 position in Text/RichText to a Unicode scalar position. |
| `resolveUtf16Position(path, pos)` | Convert a Unicode scalar position in Text/RichText to a UTF-16 position. |
| `snapshot()` | Create an immutable, pure `Snapshot` `{ revision, bytes, value }` of visible content. |
| `transact(fn)` | Execute an atomic mutation transaction on visible content; returns an immutable `Update`. |
| `applyRemote(updateOrBytes)` | Apply next server-ordered `Update` or raw `Uint8Array` bytes and rebase pending local changes. |
| `ack(updateId)` | Acknowledge pending local updates cumulatively up to `updateId` (accepts `number` or `bigint`). |
| `subscribe(subscriber)` | Subscribe to document change and error events; returns an unsubscription function `() => void`. |
| `dispose()` / `Symbol.dispose` | Release owned Wasm resources; disposal is idempotent. |

### `TransactionContext`

`doc.transact((tx: TransactionContext) => void)` provides an ergonomic, atomic mutation interface.
Parent map and list containers are automatically created if intermediate paths do not exist.

| Method | Contract |
| --- | --- |
| `tx.set(path, value)` | Replace or insert a value at the specified Path. |
| `tx.delete(path)` | Delete a key from a Map or an element from a List. |
| `tx.text(path, editOrOps)` | Mutate a Text node with a builder callback, edit step, or operations. |
| `tx.list(path, editOrOps)` | Mutate a List node with retain, insert, delete, or modify operations. |

The normal application boundary looks like this:

```ts
import { Document, text } from 'colla-ot'

const document = Document.fromJS({ title: 'Draft', content: text('Hello') })
const unsubscribe = document.subscribe({
  onChange: event => {
    if (event.origin === 'remote') editor.applyEditSteps(event.editSteps)
  },
  onError: ({ error }) => {
    console.error('Document listener failed:', error)
  },
})

const update = document.transact(tx => {
  tx.text(['content'], t => t.retain(5).insert(' world'))
})

await transport.send(update.bytes)
document.ack(update.updateId) // cumulative ACK after server acceptance

unsubscribe()
document.dispose()
```

`transact()` changes visible content immediately and emits a `'local'` change event. `Update.revision` is the
base revision captured before that local change; `Document.revision` advances
for each visible local or remote change. Acknowledgements do not emit a change
event because they do not change visible content.

### `Document` events and subscriptions

`document.subscribe(subscriber)` accepts either a direct listener function or an object with `onChange` and optional `onError`:

```ts
type DocumentChangeSubscriber = (event: DocumentChangeEvent) => void
type DocumentErrorSubscriber = (event: DocumentErrorEvent) => void
type DocumentSubscriber =
  | DocumentChangeSubscriber
  | {
      readonly onChange: DocumentChangeSubscriber
      readonly onError?: DocumentErrorSubscriber
    }

type DocumentChangeEvent = {
  readonly origin: 'local' | 'remote'
  readonly editSteps: readonly EditStep[]
  readonly revision: bigint
}

type DocumentErrorEvent = { readonly error: unknown }
```

Edit steps use Snapshot-relative paths and UTF-16 positions where a text editor
expects them. The event does not expose an owned Core `Change`. If a change
listener throws, the state transition remains committed, other listeners still
run, and the thrown value is sent to `onError` (or logged to `console.error` if omitted).

### `Snapshot`

`Snapshot` is an immutable, pure JavaScript checkpoint object representing visible content at a specific revision:

```ts
interface Snapshot {
  readonly revision: bigint
  readonly bytes: Uint8Array
  readonly value: Value
}
```

| Member | Contract |
| --- | --- |
| `Snapshot.decode(bytes)` | Strictly decode a `COLLAS` envelope from `Uint8Array` into a pure `Snapshot`. |
| `Snapshot.fromValue(value, revision?)` | Create a `Snapshot` from a Core `ValueHandle`; defaults revision to `0n`. |
| `Snapshot.fromJS(input, revision?, options?)` | Construct a `Snapshot` from structured JavaScript Value input. |
| `revision` | Stored revision as `bigint`. |
| `bytes` | Canonical binary `COLLAS` envelope as `Uint8Array`. |
| `value` | JavaScript structured Value tree. |

`Snapshot` holds no WebAssembly resources and does not require manual `dispose()`.
Restoring a Snapshot deliberately discards pending Updates, acknowledgements,
rebase state, transport state, and listeners.

### `Update`

`Update` is an immutable, pure JavaScript object carrying one versioned change and the metadata needed by the local Document queue:

```ts
interface Update {
  readonly revision: bigint
  readonly updateId: bigint
  readonly bytes: Uint8Array
}
```

| Member | Contract |
| --- | --- |
| `Update.decode(bytes)` | Strictly decode a `COLLAU` envelope from `Uint8Array` into a pure `Update`. |
| `revision` | Base revision at which the Change was created. |
| `updateId` | Per-Document `bigint` used for cumulative acknowledgement correlation. |
| `bytes` | Canonical binary `COLLAU` envelope as `Uint8Array`. |

`Update` holds no WebAssembly resources and does not require manual `dispose()`.
`updateId` starts at `1n` for each Document instance. It is not persisted in a
Snapshot and is not a globally unique operation identity.
## Values

`Value` is a closed recursive model. The JavaScript representation is:

| Kind | JavaScript representation | Notes |
| --- | --- | --- |
| Null / Bool | `null` / `boolean` | Atomic values. |
| Int | `bigint` | Signed 64-bit range; use `int()` to validate a number or bigint. |
| Float | finite `number` | NaN and infinities are rejected; negative zero normalizes to zero. |
| String | `string` | Atomic; replaced as a whole. |
| Text | `text('…')` | Character-level OT using Unicode scalar positions. |
| RichText | `richText(spans)` | Text plus atomic embeds and attribute patches. |
| List / Map | readonly array / readonly record | Maps have unique string keys. |

`ValueHandle` owns an immutable Wasm value and provides `fromJS`, `decode`,
`kind`, `has`, `get`, `toJS`, `encode`, `clone`, and `dispose`. `get()` and
`toJS()` return independently owned or recursively frozen JavaScript data; a
handle can be safely cloned before passing ownership to another subsystem.

```ts
import { ValueHandle, richText, text } from 'colla-ot'

const value = ValueHandle.fromJS({
  title: text('Draft'),
  body: richText([
    { type: 'text', text: 'Hello', attrs: { bold: true } },
    { type: 'embed', value: { id: 'mention-1' } },
  ]),
})

console.log(value.kind(['title'])) // 'text'
console.log(value.get(['title']))  // { type: 'text', value: 'Draft' }
```

### Changes

`Change` is an immutable, recursive operation relative to a base Value. It does
not carry the old value, revision, author, or operation identity. Construct it
with `Change.fromJS(input)` or the synchronous `Change.build(callback)` builder;
apply it to a concrete base to validate keys, types, and sequence ranges.

| Change kind | Builder | Operations |
| --- | --- | --- |
| `noop` | `noop()` | Identity. |
| `replace` | `replace(value)` | Replace the complete target, including its type. |
| `map` | `map(callback)` | `insert`, `delete`, or recursive `modify` by key. |
| `list` | `list(callback)` | `retain`, `insert`, `delete`, or element `modify`. |
| `text` | `text(callback)` | `retain`, `insert`, or `delete` Unicode scalars. |
| `richtext` | `richText(callback)` | Retain/format, insert text or embed, or delete. |
| `int` | `intAdd(delta)` | Checked addition (accepts safe `number` or `bigint`). |

A `Change` instance provides `kind(): ChangeKind` and `isNoop(): boolean` for
immediate inspection without needing a base Value.

```ts
const change = Change.build(builder => {
  builder.map(map => {
    map.modify('title', title => {
      title.text(textChange => textChange.retain(5).insert(' v2'))
    })
  })
})
```

The builder is a pure TypeScript input layer. It does not apply or compose
intermediate changes, own Wasm handles, or inspect a Snapshot. Constructors
normalize empty operations, merge compatible adjacent sequence operations, and
remove trailing plain retains. `Change.encode()` emits the canonical Core body.

### OT operations

```ts
import {
  apply,
  compose,
  invert,
  transformPair,
} from 'colla-ot'

const after = apply(base, change)
const combined = compose(first, second)
const inverse = invert(combined, base)
const [leftPrime, rightPrime] = transformPair(left, right, {
  order: 'left-first',
})
```

| Function | Meaning | Important precondition |
| --- | --- | --- |
| `apply(base, change)` | Return a new Value after the operation. | Change must match the concrete base. |
| `compose(first, second)` | Combine sequential operations into one. | `second` applies after `first`. |
| `invert(change, base)` | Produce an operation that restores `base`. | The original base is required because old values are not stored in Change. |
| `transformPair(left, right, options)` | Transform concurrent operations from one base. | Choose a deterministic `left-first` or `right-first` tie-break. |

These functions never consume their inputs. Use `inspectChange(change, base)` for
a read-only structural view and `convertChangeToEditSteps(change, base)` for an
editor projection. Neither projection is valid Change input or a persistence
format.

### Coordinates

Core Text and RichText operations count Unicode scalar values. JavaScript editor
positions count UTF-16 code units, so use explicit conversion at the boundary:

```ts
const value = ValueHandle.fromJS(text('A😀B'))
resolveCodePointPosition(value, [], 3) // 2
resolveUtf16Position(value, [], 2)     // 3
```

Positions inside a surrogate pair are rejected with
`invalid_utf16_boundary`. RichText embeds count as one sequence unit in both
coordinate systems.

## Limits, errors, and ownership

### Structured input limits

Pass `InputOptions` to `ValueHandle.fromJS`, `Change.fromJS`, or `Change.build`
when parsing untrusted JavaScript objects. Limits are counted before semantic
normalization, so empty operations cannot bypass policy.

| Field | Default | Protects |
| --- | ---: | --- |
| `maxDepth` | `128` | Recursive Value/Change depth. |
| `maxValueNodes` | `1,000,000` | Value node count. |
| `maxChangeNodes` | `1,000,000` | Change node count. |
| `maxContainerLength` | `1,000,000` | Map/List/attribute entries. |
| `maxStringBytes` | `16 MiB` | One UTF-8 string. |
| `maxSequenceOps` | `1,000,000` | Raw sequence operation count. |
| `maxSequenceLength` | `1,000,000` | Logical sequence length. |

These limits apply to structured input only. Canonical byte decoding uses the
codec's own recursion and allocation defenses; OT algebra and projections do
not read `InputLimits`.

### `CollaError`

Public failures throw `CollaError` with stable `code`, `operation`, optional
Snapshot-relative `path`, and frozen `details`. Match `code` rather than human
message text. The shared codes are documented in [Glossary and errors](/reference/glossary).

```ts
import { CollaError, ValueHandle } from 'colla-ot'

try {
  ValueHandle.decode(bytes)
} catch (error) {
  if (error instanceof CollaError && error.is('invalid_encoding')) {
    console.error(error.operation, error.details)
  }
}
```

### Resource lifecycle

`Document`, `ValueHandle`, and `Change` own Wasm-backed resources and provide `dispose()`
(and `Symbol.dispose`) for deterministic release. Cloned `ValueHandle` instances have
independent ownership. Disposal is idempotent; invoking methods on a disposed handle throws `invalid_state`.

In contrast, `Snapshot` and `Update` are pure, immutable JavaScript values. They hold no
WebAssembly handles and require zero manual memory management, relying entirely on standard
JavaScript garbage collection.

## Related pages

- [Document state](/docs/document/) — complete application state flows.
- [Document synchronization](/docs/examples/javascript-document) — persistence, events, and sync.
- [Data model](/docs/core/values) — Value, Change, Text, and RichText semantics.
- [OT guide](/docs/ot/) — algebra, rebasing, and TP1/TP2 boundaries.
- [Rust API](/reference/rust) — the reference crate surface.
- [Protocol reference](/reference/protocol) — canonical bodies and envelopes.
- [Published npm package](https://www.npmjs.com/package/colla-ot)
- [TypeScript source](https://github.com/link-duan/colla/tree/master/packages/core)
