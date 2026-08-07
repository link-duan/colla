# @colla/core

Synchronous JavaScript bindings for Colla's immutable nested values and
Operational Transformation algebra. The same canonical codec and OT logic are
used by Rust and JavaScript.

## Install

```sh
pnpm add @colla/core
```

Node.js 22 or newer is supported through ESM. Vite 5 or newer and Rollup 4 or
newer can consume the same package with ordinary module resolution; no Wasm,
top-level-await or asset copy plugin is required. The corresponding Rust crate
supports Rust 1.81 or newer.

The Rust crate and npm package share one version. Patch releases in the 0.1
line preserve public API and wire compatibility; a later pre-1.0 minor may make
documented breaking changes. See the repository changelog and release runbook
for the complete policy and supported release procedure.

## Values and changes

```ts
import { Value, apply, richText, text } from "@colla/core"

const before = Value.fromJS({
  count: 1n,
  title: text("Draft"),
  body: richText([{ type: "text", text: "Hello" }]),
})

const change = before.change()
  .int(["count"], value => value.add(1n))
  .text(["title"], value => value.insert(5, " v2"))
  .richText(["body"], value => value
    .insertText(5, " world", { bold: true }))
  .build()

const after = apply(before, change)
console.log(after.toJS())

before.dispose()
change.dispose()
after.dispose()
```

`ValueInput` accepts null, booleans, signed 64-bit `bigint`, finite numbers,
strings, arrays, plain records, `text()` and `richText()`. Output from `toJS()`
and `inspectChange()` is recursively frozen.

Builders are sequential and transactional. Scoped callbacks must be
synchronous and cannot escape. RichText exposes typed text/embed insertion,
half-open deletion and explicit attribute formatting; embeds are atomic.
Text and RichText Builder coordinates use JavaScript UTF-16 code units.

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
} from "@colla/core"

const value = Value.decode(valueBytes)
const first = Change.decode(firstBytes)
const second = Change.decode(secondBytes)
const concurrent = Change.decode(concurrentBytes)

const combined = compose(first, second)
const inverse = invert(combined, value)
const [leftPrime, rightPrime] = transformPair(first, concurrent, {
  order: "left-first",
})
const view = inspectChange(combined, value)
const next = apply(value, combined)

for (const handle of [
  value,
  first,
  second,
  concurrent,
  combined,
  inverse,
  leftPrime,
  rightPrime,
  next,
]) handle.dispose()
```

`encode()` returns canonical bytes owned by JavaScript. Algebra never consumes
its inputs and every returned `Value` or `Change` has independent disposal
ownership. `ChangeView` is for inspection only; it is not a Change construction
or persistence format.

## Input limits and errors

`InputLimits` apply only at untrusted input boundaries:

- `Value.fromJS(input, { limits })`
- `Value.decode(bytes, { limits })`
- `Change.decode(bytes, { limits })`

Builder inputs are validated for Core semantics but trusted for size. Builders,
algebra, coordinate conversion and Change inspection do not apply limits.

Failures throw `CollaError`. Use stable `code`, `operation`, optional `path` and
frozen `details` fields rather than matching message text.

```ts
import { CollaError, Value } from "@colla/core"

try {
  Value.decode(bytes)
} catch (error) {
  if (error instanceof CollaError && error.is("invalid_encoding")) {
    console.error(error.operation, error.details)
  }
}
```

## Node.js, Vite, Rollup and workers

Node.js uses the normal package import:

```ts
import { Value } from "@colla/core"
```

Vite and Rollup use the same source import. Keep their standard ESM and module
resolution settings; no package-specific plugin or asynchronous initialization
step is needed.

Dedicated and Shared Worker modules can import the package directly:

```ts
import { Value } from "@colla/core"

const value = Value.fromJS({ worker: true })
const result = value.toJS()
value.dispose()
```

Initialization does not depend on `document`, `window` or another DOM API.

## Resource lifecycle

Call `dispose()` (or use `Symbol.dispose`) as soon as a `Value`, `Change` or root
Builder is no longer needed. Disposal is idempotent. `FinalizationRegistry` is
only a fallback for missed cleanup and must not be the primary lifecycle plan.
