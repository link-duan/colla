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
import { Value, richText, text } from "colla-ot"

using value = Value.fromJS({
  count: 1n,
  title: text("Draft"),
  body: richText([
    { type: "text", text: "Hello", attrs: { bold: true } },
    { type: "embed", value: { id: "mention-1" } },
  ]),
})

console.log(value.toJS())
```

`ValueInput` accepts null, booleans, signed 64-bit `bigint`, finite numbers,
strings, arrays, plain records, `text()` markers, and `richText()` markers.
`ValueData` returned by `get()` and `toJS()` is recursively frozen; map output
uses null-prototype records.

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
import { Change, Value, apply, text } from "colla-ot"

using before = Value.fromJS({
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
  Value,
  resolveCodePointPosition,
  resolveUtf16Position,
  text,
} from "colla-ot"

using value = Value.fromJS(text("A😀B"))

resolveCodePointPosition(value, [], 3) // 2
resolveUtf16Position(value, [], 2)     // 3

using change = Change.build(change => {
  change.text(text => text.retain(2).insert("X"))
})
```

UTF-16 positions inside a surrogate pair are rejected. `inspectChange()` uses
Snapshot-relative UTF-16 positions for JavaScript-facing views.

## Codec and algebra

```ts
import {
  Change,
  Value,
  apply,
  compose,
  inspectChange,
  invert,
  transformPair,
} from "colla-ot"

using value = Value.decode(valueBytes)
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

leftPrime.dispose()
rightPrime.dispose()
```

`encode()` returns fresh JavaScript-owned canonical bytes. Algebra never
consumes its inputs. `ChangeView` is a read-only projection for inspection; it
is neither Change construction data nor a persistence format.

## Input limits and errors

`InputOptions` may override `DEFAULT_INPUT_LIMITS` at untrusted input
boundaries:

- `Value.fromJS(input, { limits })`
- `Value.decode(bytes, { limits })`
- `Change.fromJS(input, { limits })`
- `Change.build(edit, { limits })`
- `Change.decode(bytes, { limits })`

Change construction limits are counted against raw input before normalization,
so empty operations cannot bypass resource policy. Algebra, coordinate
conversion, and Change inspection do not apply input limits.

Failures throw `CollaError`. Match its stable `code`, `operation`, optional
`path`, and frozen `details` fields rather than message text.

```ts
import { CollaError, Value } from "colla-ot"

try {
  Value.decode(bytes)
} catch (error) {
  if (error instanceof CollaError && error.is("invalid_encoding")) {
    console.error(error.operation, error.details)
  }
}
```

## Resource lifecycle

`Value` and `Change` are Wasm-backed handles. Call `dispose()` or use
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
