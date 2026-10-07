# JavaScript API

Import from `colla-ot` in Node.js 22+, browsers and workers. The package selects its
platform entry through ESM exports. All editing and codec calls are synchronous; no
public Wasm initialization, handle cloning or disposal is required.

Use the [tutorials](/docs/getting-started/) for complete workflows and the
[two-client example](/docs/examples/sync) for checked code. This page lists the public
surface; optional results are written explicitly. Type declarations below describe API
shapes and are not standalone programs.

## Input and paths

| Type or helper                                       | Definition / result                                                                                         |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Input                                                | null, boolean, bigint, finite number, string, Text, RichText, Value, readonly Input array or plain InputMap |
| Path                                                 | readonly array of string keys and number indexes; `[]` is the root                                          |
| `text(value: string): Text`                          | Immutable collaborative text wrapper; also `new Text(value)`                                                |
| `richText(spans: readonly RichTextSpan[]): RichText` | Immutable formatted sequence; also `new RichText(spans)`                                                    |
| AttrValue                                            | boolean, bigint, finite number or string                                                                    |
| Attrs / AttrPatch                                    | String-keyed attributes; patches also permit null for removal                                               |

Text exposes `type: 'text'` and `value: string`.
RichText exposes `type: 'richtext'` and readonly `spans`. A RichTextSpan is
`{ type: 'text', text: string, attrs? }` or `{ type: 'embed', value: Input, attrs? }`.
Int is signed i64 represented by bigint; number represents finite Float.

## Value and shared reads

| Member                                   | Result / behavior                                            |
| ---------------------------------------- | ------------------------------------------------------------ |
| `Value.fromJS(input: Input): Value`      | Construct immutable content; a Value input is returned as-is |
| `Value.decode(bytes: Uint8Array): Value` | Strict decode                                                |
| `encode(): Uint8Array`                   | Fresh independent canonical bytes                            |
| `toJS(): Input`                          | Immutable content projection                                 |
| `equals(other: Value): boolean`          | Structural equality                                          |

Value, Document and Transaction share these reads. Document/Transaction operate on their
current or working snapshot; Value operates on its own immutable content.

| Read                                        | Result                                                      |
| ------------------------------------------- | ----------------------------------------------------------- |
| `get(path?: Path): Value \| undefined`      | Root by default; absent target returns undefined            |
| `has(path: Path): boolean`                  | Whether the target exists                                   |
| `kind(path?: Path): ValueKind \| undefined` | null, bool, int, float, string, text, richtext, list or map |

For `get`, `kind` and `has`, only `missing_key` and `out_of_bounds` become an absent
result (`undefined` or `false`). A Path that traverses a scalar or uses a segment
incompatible with the current container throws `type_mismatch`, even if its argument
shape is valid. Invalid arguments and lifecycle errors also propagate. See
[Paths](/docs/core/paths).

## Change and operations

| Member                                                    | Result                         |
| --------------------------------------------------------- | ------------------------------ |
| `Change.create(operations: readonly Operation[]): Change` | Validated ordered sequence     |
| `Change.noop(): Change`                                   | Empty sequence                 |
| `Change.decode(bytes: Uint8Array): Change`                | Strict decode                  |
| `change.isNoop: boolean`                                  | Whether the sequence is empty  |
| `change.operations: readonly Operation[]`                 | Immutable operation projection |
| `change.encode(): Uint8Array`                             | Canonical bytes                |

Every operation has a `path`. Insert's path ends with a List insertion index; Set
writes a Map member, adding it if missing, or replaces a List item or the root.
listMove's path names the List, and `to` counts after removal.

```ts
type Operation =
  | { readonly type: 'insert'; readonly path: Path; readonly value: Value }
  | { readonly type: 'delete'; readonly path: Path }
  | { readonly type: 'set'; readonly path: Path; readonly value: Value }
  | { readonly type: 'listMove'; readonly path: Path; readonly from: number; readonly to: number }
  | { readonly type: 'text'; readonly path: Path; readonly operations: readonly TextOp[] }
  | { readonly type: 'add'; readonly path: Path; readonly delta: bigint }
  | { readonly type: 'richtext'; readonly path: Path; readonly operations: readonly RichTextOp[] }
```

TextOp is retain/delete with a numeric length, or insert with a string text.
RichTextOp is retain with length and optional AttrPatch, delete with length, or insert
with a RichTextSpan. All low-level sequence lengths count Unicode scalars; embeds count
one. Operations run in order against the content produced by previous operations.

## Algebra

| Function                                                                                        | Return                    |
| ----------------------------------------------------------------------------------------------- | ------------------------- |
| `apply(base: Value, change: Change)`                                                            | Value                     |
| `invert(base: Value, change: Change)`                                                           | Change                    |
| `compose(base: Value, first: Change, second: Change)`                                           | Change                    |
| `transform(base: Value, left: Change, right: Change, options: { priority: 'left' \| 'right' })` | readonly [Change, Change] |

Transform returns **left-after-right first, right-after-left second**. Both inputs share
a base. Compose's second input applies after the first. Invert restores content when
applied after change. TP1 holds for all valid inputs. See
[Changes](/docs/core/changes) and
[Change algebra](/docs/core/algebra).

## Document and Transaction

| Document member                                                               | Result / behavior                          |
| ----------------------------------------------------------------------------- | ------------------------------------------ |
| `Document.create(input: Input): Document`                                     | Standalone runtime                         |
| `snapshot(): Value`                                                           | Immutable current content                  |
| `version: bigint`                                                             | Local content version, not server revision |
| `edit(callback: (tx: Transaction) => unknown, options?: { group?: string })`  | EditResult or null for Noop                |
| `apply(change: Change)`                                                       | EditResult or null                         |
| `subscribe(listener: (event: EditEvent) => void, options?: SubscribeOptions)` | Unsubscribe function                       |
| `close(): void`                                                               | Idempotent close                           |

Transaction has shared reads, `snapshot(): Value`, and these scoped mutations:

| Transaction member                     | Return         |
| -------------------------------------- | -------------- |
| `set(path: Path, input: Input)`        | void           |
| `delete(path: Path)`                   | void           |
| `increment(path: Path, delta: bigint)` | void           |
| `apply(change: Change)`                | void           |
| `list(path: Path)`                     | ListEditor     |
| `text(path: Path)`                     | TextEditor     |
| `richText(path: Path)`                 | RichTextEditor |

Callbacks must be synchronous: no thenables, nested edits, remote receive or close.
Escaped transactions/editors are invalid after callback exit. All failures roll back the
transaction. Each call interprets its Path against the working content. See [Transactions](/docs/editing/transactions) and
[Map and List editing](/docs/editing/maps-lists).

## Scoped sequence editors

All methods return void and validate the target kind and ranges.

| Editor         | Signatures                                                                                                                                                                                                                     |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| ListEditor     | `insert(index, values: readonly Input[])`, `delete(index, count)`, `replace(index, count, values: readonly Input[])`, `move(from, to)`                                                                                         |
| TextEditor     | `insert(index, text: string)`, `delete(index, count)`, `replace(index, count, text: string)`                                                                                                                                   |
| RichTextEditor | `insertText(index, text: string, attrs?: Attrs)`, `insertEmbed(index, value: Input, attrs?: Attrs)`, `delete(index, count)`, `replace(index, count, spans: readonly RichTextSpan[])`, `format(index, count, patch: AttrPatch)` |

Indexes and counts are numbers. High-level Text/RichText methods use UTF-16 units in
the working content at each step. Splitting a surrogate pair fails; RichText embeds
occupy one position. List counts are element counts. See [Positions](/docs/core/positions).

## Results and observation

EditEvent is EditResult. All returned data is immutable.

```ts
interface EditResult {
  readonly before: Value
  readonly after: Value
  readonly change: Change
  readonly inverse: Change
  readonly version: bigint
  readonly origin: 'local' | 'remote' | 'undo' | 'redo'
}
interface SubscribeOptions {
  readonly onError?: (error: unknown) => void
}
```

Events run synchronously after commit. Listener exceptions are isolated; onError handles
listener diagnostics. Reads during dispatch are allowed, mutation is rejected. Noop edits
produce no event. Returned events and snapshots survive runtime close.

## History

| Member                                                           | Result / behavior                                            |
| ---------------------------------------------------------------- | ------------------------------------------------------------ |
| `History.attach(doc: Document, options?: { capacity?: number })` | History; existing instance if attached; default capacity 100 |
| `History.restore(doc: Document, checkpoint: HistoryCheckpoint)`  | History; requires exact content basis                        |
| `canUndo`, `canRedo`                                             | boolean getters                                              |
| `undo()`, `redo()`                                               | EditResult or null                                           |
| `clear()`                                                        | void; empties stacks                                         |
| `checkpoint()`                                                   | HistoryCheckpoint                                            |
| `close()`                                                        | void; idempotent detach                                      |

Equal consecutive explicit edit groups merge. Remote commits and undo/redo end groups.
Remote changes rebase history with remote priority; new local editing clears redo.
Undo/redo are synchronized as new local edits. See [History](/docs/history/).

## SyncSession

| Member                                                                        | Result / behavior                                 |
| ----------------------------------------------------------------------------- | ------------------------------------------------- |
| `SyncSession.create({ clientId: string, snapshot: SyncSnapshot })`            | SyncSession                                       |
| `SyncSession.restore(checkpoint: SessionCheckpoint)`                          | SyncSession with original writer identity         |
| `document`                                                                    | Document for local editing                        |
| `state`                                                                       | SyncState                                         |
| `revision`                                                                    | bigint confirmed revision                         |
| `outbound()`                                                                  | Submission or null; retries retain original bytes |
| `receive(message: ServerMessage)`                                             | EditResult or null; can throw or enter recovery   |
| `checkpoint()`                                                                | SessionCheckpoint, including enabled History      |
| `subscribe(listener: (state: SyncState) => void, options?: SubscribeOptions)` | Unsubscribe function                              |
| `close()`                                                                     | void; closes its Document and listeners           |

SyncState contains status (`active`, `recovery-required`, `closed`), revision: bigint,
hasOutbound: boolean, and optional recoveryReason: CollaError. Observe state for formal
confirmation without a content event. Missing revision errors report the required interval;
recovery-required retains work and halts sending. See [Sync](/docs/sync/).

## Authority and controlled objects

| Member                                                   | Result                                                               |
| -------------------------------------------------------- | -------------------------------------------------------------------- |
| `Authority.create({ documentId: string, value: Input })` | Authority                                                            |
| `Authority.restore(checkpoint: AuthorityCheckpoint)`     | Authority                                                            |
| `revision`                                               | bigint                                                               |
| `snapshot()`                                             | SyncSnapshot                                                         |
| `accept(submission: Submission)`                         | `{ readonly authority: Authority; readonly message: ServerMessage }` |
| `commitsSince(revision: bigint)`                         | readonly ServerMessage[]                                             |
| `compact(throughRevision: bigint)`                       | New Authority                                                        |
| `checkpoint()`                                           | AuthorityCheckpoint                                                  |

Authority is immutable. Persist returned state before adoption and broadcast. Protocol
and checkpoint classes are constructed by runtimes or static decode, not plain field
objects. All expose encode(): Uint8Array and static decode(bytes).
See [Protocol and encoding](/reference/protocol) for fields, tags and validation.

## Errors and lifecycle

CollaError extends Error and exposes code: ErrorCode, operation: string, immutable
details and optional elementId. Match the [stable codes](/reference/glossary#error-codes),
not reason text. Decoding rejects malformed, wrong-version or oversized bytes.

Close runtimes when finished. Value, Change, wrappers, events and protocol objects do
not expose free/dispose. Transactions expire at the end of their callback, independently
of Document lifetime. See [Lifecycle](/docs/editing/lifecycle).
