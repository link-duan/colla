# colla-ot

`colla-ot` is the synchronous JavaScript facade for Colla's immutable nested
values, canonical Change format, and Operational Transformation algebra. It
uses the same Rust core and canonical bytes as the `colla` crate.

## Install

```sh
pnpm add colla-ot
```

The package is ESM-only and supports Node.js 22+, Vite 5+, and Rollup 4+.
Browser and Node entry points initialize the same Wasm binary synchronously;
no public initialization function or Wasm bundler plugin is required.

## Values

```ts
import { ValueHandle, richText, text } from "colla-ot"

using value = ValueHandle.fromJS({
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
import { Change } from "colla-ot"

using change = Change.fromJS({
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
import { Change, ValueHandle, apply, text } from "colla-ot"

using before = ValueHandle.fromJS({
  count: 1n,
  title: text("Draft"),
})

using change = Change.build(change => {
  change.map(map => {
    map.modify("count", count => count.intAdd(1n))
    map.modify("title", title => {
      title.text(text => text.retain(5).insert(" v2"))
    })
  })
})

using after = apply(before, change)
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
} from "colla-ot"

using value = ValueHandle.fromJS(text("A😀B"))

resolveCodePointPosition(value, [], 3) // 2
resolveUtf16Position(value, [], 2)     // 3

using change = Change.build(change => {
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
} from "colla-ot"

using value = ValueHandle.decode(valueBytes)
using first = Change.decode(firstBytes)
using second = Change.decode(secondBytes)
using concurrent = Change.decode(concurrentBytes)

using combined = compose(first, second)
using inverse = invert(combined, value)
using next = apply(value, combined)
const [leftPrime, rightPrime] = transformPair(first, concurrent, {
  order: "left-first",
})
const view = inspectChange(combined, value)
const steps = convertChangeToEditSteps(combined, value)

leftPrime.dispose()
rightPrime.dispose()
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

Failures throw `CollaError`. Match its stable `code`, `operation`, optional
`path`, and frozen `details` fields rather than message text.

```ts
import { CollaError, ValueHandle } from "colla-ot"

try {
  ValueHandle.decode(bytes)
} catch (error) {
  if (error instanceof CollaError && error.is("invalid_encoding")) {
    console.error(error.operation, error.details)
  }
}
```

## Resource lifecycle

`ValueHandle` and `Change` are Wasm-backed handles. Call `dispose()` or use
`Symbol.dispose` as soon as ownership ends. Disposal is idempotent, and clones
have independent ownership. `FinalizationRegistry` is only a fallback for
missed cleanup.

The callback builders used by `Change.build()` are ordinary TypeScript scopes;
they have no handle, clone, finalizer, or disposal API.

## More documentation

See the repository
[documentation index](https://github.com/link-duan/colla/blob/master/docs/README.md)
for the data model, OT properties, binary format, architecture decisions,
compatibility policy, and release process.
