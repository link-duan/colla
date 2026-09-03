# colla-ot

`colla-ot` provides two API levels backed by the same Rust core and canonical
Value/Change bytes:

- import `Document`, `Snapshot`, and `Update` from `colla-ot` when building an
  editor or application that needs mutable document state;
- import immutable values, changes, codecs, and OT algebra from `colla-ot/core`
  when implementing lower-level operations.

## Install

```sh
pnpm add colla-ot
```

The package is ESM-only and supports Node.js 22+, Vite 5+, and Rollup 4+.
Browser and Node entry points initialize the same Wasm binary synchronously;
no public initialization function or Wasm bundler plugin is required. Importing
both the package root and `colla-ot/core` shares the same runtime initialization.

## High-level Document API

`Document` is the usual entry point for an application. It owns the current
visible content, applies local edits optimistically, rebases them over ordered
remote updates, and emits editor-oriented change events.

### Create a document

Create a new document from a structured Core Value. Use `text()` for content
that needs character-level editing; ordinary JavaScript strings are atomic.

```ts
import { Document } from "colla-ot"
import { text } from "colla-ot/core"

const document = Document.fromJS({
  title: text("Draft"),
  published: false,
})
```

The optional second argument sets the initial revision and defaults to `0n`.

To read the current content, request an independently owned `ValueHandle`:

```ts
const value = document.value()
console.log(value.toJS())
console.log(document.revision)
```

### Restore a persisted Snapshot

`Snapshot` stores the complete visible Core Value together with its revision.
Decode the bytes and create a new Document from it:

```ts
import { Document, Snapshot } from "colla-ot"

const bytes = await storage.read("document.snapshot")
const snapshot = Snapshot.decode(bytes)
const document = Document.fromSnapshot(snapshot)
```

The Document owns a clone of the Snapshot content, so the Snapshot does not
need to stay reachable after construction.

### Subscribe to changes

Subscribe before applying updates when an editor or another view must follow
the Document state:

```ts
const unsubscribe = document.on("change", event => {
  if (event.origin === "remote") {
    editor.applyEditSteps(event.editSteps)
  }
  console.log("visible revision", event.revision)
})

document.on("error", event => {
  console.error("Document listener failed", event.error)
})
```

A change event contains only `origin`, `revision`, and immutable `editSteps`.
It does not transfer ownership of a Core Change. Use `origin` to avoid applying
an edit back to the editor that originally produced it. Calling the returned
function removes that listener:

```ts
unsubscribe()
```

If a change listener throws, the state change remains committed, other change
listeners still run, and the exception is delivered to error listeners. An
error listener failure is ignored to prevent recursive error reporting.

### Apply and send a local edit

Build the Core Change with `colla-ot/core`, then apply it to the Document. The
visible content changes immediately and `applyLocal()` returns the Update to
send to the application server:

```ts
import { Change } from "colla-ot/core"

const change = Change.build(change => {
  change.map(map => {
    map.modify("title", title => {
      title.text(text => text.retain(5).insert(" v2"))
    })
  })
})

const update = document.applyLocal(change)
await transport.send(update.encode())
```

Each Update contains its base revision, an instance-local `updateId`, and one
Core Change. `updateId` starts at `1` for each Document instance and is only a
local correlation value; it is not a globally unique operation identity.

After the server accepts the oldest pending local Update, acknowledge it by ID:

```ts
document.ack(update.updateId)
```

Acknowledgements must follow pending order. An acknowledgement advances the
confirmed state but does not emit a change event because visible content does
not change.

### Apply a remote update

Decode each server-ordered Update and pass it to the Document:

```ts
import { Update } from "colla-ot"

const bytes: Uint8Array = await transport.receive()
const update = Update.decode(bytes)
document.applyRemote(update)
```

The incoming Update must be based on the next confirmed revision. Document
rebases pending local changes over it with a fixed `left-first` tie-break and
emits one remote change event for the resulting visible edit.

The server or another application-level protocol is responsible for ordering
remote Updates. Document does not provide transport, sessions, retries,
presence, global deduplication, or peer-to-peer convergence.

### Persist current content

Create and encode a Snapshot whenever the application needs a content
checkpoint:

```ts
const snapshot = document.snapshot()
await storage.write("document.snapshot", snapshot.encode())
```

The Snapshot contains the current visible content and revision. It deliberately
does not contain pending Updates, acknowledgements, rebase state, transport
state, or listeners. Snapshot bytes alone are therefore not a resumable sync
session. Applications that need crash-safe delivery must define an additional
outbound-queue and recovery protocol outside the current Document API.

Snapshot and Update use distinct versioned binary envelopes. They are intended
for local persistence and application transport, while raw Value and Change
bytes remain the lower-level Core formats. Colla is still in early development
and does not promise compatibility with historical Snapshot/Update bytes.

Most consumers obtain Snapshots from `document.snapshot()` and local Updates
from `document.applyLocal()`. At other boundaries, `Snapshot.fromJS()`,
`Snapshot.fromValue()`, and `Update.fromChange()` construct envelopes directly.
`snapshot.content()` and `update.change()` return independently owned handles.

## Low-level Core API

The remaining APIs operate on immutable Core Values and Changes without owning
Document state. Import them from `colla-ot/core`.

## Values

```ts
import { ValueHandle, richText, text } from "colla-ot/core"

const value = ValueHandle.fromJS({
  count: 1n,
  title: text("Draft"),
  body: richText([
    { type: "text", text: "Hello", attrs: { bold: true } },
    { type: "embed", value: { id: "mention-1" } },
  ]),
})

console.log(value.toJS())
```

The recursive `Value` type covers null, booleans, signed 64-bit `bigint`, finite
numbers, strings, arrays, plain records, `text()` markers, and `richText()`
markers. `ValueHandle.fromJS()` accepts a `Value`; `get()` and `toJS()` return
the same type. Returned values are recursively frozen, and map output uses
null-prototype records. The RichText marker uses `type: "richtext"`.

Ordinary strings are atomic. Use `text()` when character-level OT is required.
RichText embeds are atomic Core Values and count as one sequence unit.

## Typed Change construction

`Change.fromJS()` is the low-level, Snapshot-independent construction API.
Map entries remain an array so duplicate keys can be rejected, while sequence
changes use ordered operation streams.

```ts
import { Change } from "colla-ot/core"

const change = Change.fromJS({
  type: "map",
  entries: [{
    key: "title",
    type: "modify",
    change: {
      type: "text",
      ops: [
        { type: "retain", length: 5 },
        { type: "insert", text: " v2" },
      ],
    },
  }],
})
```

Construction does not inspect a Snapshot. Key existence, target types, and
sequence bounds are checked when the Change is applied.

## TypeScript Builder

`Change.build()` is a pure TypeScript convenience layer over the same typed
input. It does not own a Wasm handle, apply or compose intermediate changes,
parse paths, perform map upserts, or convert coordinates.

```ts
import { Change, ValueHandle, apply, text } from "colla-ot/core"

const before = ValueHandle.fromJS({
  count: 1n,
  title: text("Draft"),
})

const change = Change.build(change => {
  change.map(map => {
    map.modify("count", count => count.intAdd(1n))
    map.modify("title", title => {
      title.text(text => text.retain(5).insert(" v2"))
    })
  })
})

const after = apply(before, change)
```

Each root or nested Change callback must select exactly one Change kind.
Scoped builders are synchronous and close when their callback returns or
throws; they cannot be retained for later mutation. Map operations explicitly
choose `insert`, `delete`, or `modify`.

Raw operation streams may contain zero-length operations, empty inserts,
adjacent operations, or trailing retains. Rust typed constructors perform the
canonical normalization and checked length accumulation.

## Text coordinates

Text and RichText `retain` and `delete` lengths use Unicode scalar values, not
JavaScript UTF-16 code units. RichText embeds count as one in both coordinate
systems.

```ts
import {
  Change,
  ValueHandle,
  resolveCodePointPosition,
  resolveUtf16Position,
  text,
} from "colla-ot/core"

const value = ValueHandle.fromJS(text("A😀B"))

resolveCodePointPosition(value, [], 3) // 2
resolveUtf16Position(value, [], 2)     // 3

const change = Change.build(change => {
  change.text(text => text.retain(2).insert("X"))
})
```

UTF-16 positions inside a surrogate pair are rejected. `inspectChange()` and
`convertChangeToEditSteps()` use Snapshot-relative UTF-16 positions for
JavaScript-facing projections.

## Codec and algebra

```ts
import {
  Change,
  ValueHandle,
  apply,
  compose,
  convertChangeToEditSteps,
  inspectChange,
  invert,
  transformPair,
} from "colla-ot/core"

const value = ValueHandle.decode(valueBytes)
const first = Change.decode(firstBytes)
const second = Change.decode(secondBytes)
const concurrent = Change.decode(concurrentBytes)

const combined = compose(first, second)
const inverse = invert(combined, value)
const next = apply(value, combined)
const [leftPrime, rightPrime] = transformPair(first, concurrent, {
  order: "left-first",
})
const view = inspectChange(combined, value)
const steps = convertChangeToEditSteps(combined, value)

```

`encode()` returns fresh JavaScript-owned canonical bytes. Algebra never
consumes its inputs. `ChangeView` is a read-only projection for inspection; it
is neither Change construction data nor a persistence format.

`convertChangeToEditSteps()` is the editor-oriented projection. It retains
List, Text, and RichText operation streams instead of flattening them into
positioned events. Map insert/delete steps use the child key path, Map modify
recurses, and List modify contains element-relative nested steps. The returned
array, steps, paths, operations, patches, and Values are recursively frozen.
Edit Steps are runtime projections, not Change input or a wire/persistence
format.

## Input limits and errors

`InputOptions` may override `DEFAULT_INPUT_LIMITS` at untrusted input
boundaries:

- `ValueHandle.fromJS(input, { limits })`
- `Change.fromJS(input, { limits })`
- `Change.build(edit, { limits })`

Change construction limits are counted against raw input before normalization,
so empty operations cannot bypass resource policy. Algebra, coordinate
conversion, and Change inspection do not apply input limits.

Failures from both API levels throw `CollaError`. Match its stable `code`,
`operation`, optional `path`, and frozen `details` fields rather than message
text. `CollaError` is exported from `colla-ot/core`.

```ts
import { CollaError, ValueHandle } from "colla-ot/core"

try {
  ValueHandle.decode(bytes)
} catch (error) {
  if (error instanceof CollaError && error.is("invalid_encoding")) {
    console.error(error.operation, error.details)
  }
}
```

## Resource lifecycle

`ValueHandle`, `Change`, `Snapshot`, and `Update` own Wasm-backed resources;
`Document` owns such handles internally. In normal JavaScript usage, unreachable
objects are reclaimed by JS garbage collection and the wasm-bindgen-generated
finalizers. GC timing is nondeterministic, so applications with high allocation
churn, long-running processes, or strict allocation peaks may call the optional
`dispose()` method (or `Symbol.dispose`) when ownership ends. Disposal is
idempotent, and clones have independent ownership.
In runtimes without `FinalizationRegistry`, call `dispose()` when resources must
be released, because the generated fallback cannot observe unreachable objects.

The callback builders used by `Change.build()` are ordinary TypeScript scopes;
they have no handle, clone, finalizer, or disposal API.

## More documentation

See the repository
[documentation index](https://github.com/link-duan/colla/blob/master/docs/README.md)
for the normative Document model, Core data model, OT properties, binary
formats, architecture decisions, compatibility policy, and release process.
