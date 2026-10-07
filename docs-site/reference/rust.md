# Rust API

Use `colla` for immutable content, transactional editing, History and synchronization.
Requires Rust 1.81 or newer. See [Getting started](/docs/getting-started/) for installation
and [Rust examples](/docs/examples/rust) for runnable programs.

Signatures below omit the receiver: reads use `&self`, Transaction mutations use
`&mut self`, and functions prefixed with a type name are associated functions.
`Result<T>` means `std::result::Result<T, CollaError>`. These tables are API lookup,
not standalone programs.

## Content types

| Type      | Variants or fields                                                                                                                                              |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Body      | `Null`, `Bool(bool)`, `Int(i64)`, `Float(f64)`, `String(String)`, `Text(String)`, `RichText(Vec<RichSpan>)`, `List(Vec<Value>)`, `Map(BTreeMap<String, Value>)` |
| RichSpan  | `Text { text: String, attrs: Attrs }`, `Embed { value: Value, attrs: Attrs }`                                                                                   |
| Attr      | `Bool(bool)`, `Int(i64)`, `Float(f64)`, `String(String)`                                                                                                        |
| Attrs     | `BTreeMap<String, Attr>`                                                                                                                                        |
| AttrPatch | `BTreeMap<String, Option<Attr>>`; None removes an attribute                                                                                                     |
| Path      | `Vec<Segment>`; empty for the root                                                                                                                              |
| Segment   | `Key(String)`, `Index(usize)`                                                                                                                                   |

Value clones share immutable storage; equality is structural.
Float values must be finite. See [Values](/docs/core/values) for the content model.

## Value methods

| Constructor or method                                                                                                                                                         | Return                             |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- |
| `Value::new(body: Body)`                                                                                                                                                      | Result&lt;Value&gt;                |
| `Value::null()`, `bool(bool)`, `int(i64)`                                                                                                                                     | Value                              |
| `Value::string(impl Into<String>)`, `text(impl Into<String>)`, `float(f64)`, `list(Vec<Value>)`, `map(impl IntoIterator<Item = (String, Value)>)`, `rich_text(Vec<RichSpan>)` | Result&lt;Value&gt;                |
| `body()`, `kind()`                                                                                                                                                            | &Body, &'static str                |
| `get(&[Segment])`                                                                                                                                                             | Result&lt;&Value&gt;               |
| `has(&[Segment])`                                                                                                                                                             | bool                               |
| `encode()`, `Value::decode(&[u8])`                                                                                                                                            | Vec&lt;u8&gt;, Result&lt;Value&gt; |

Unlike JavaScript get, Rust get reports missing values as an error.

## Change methods and types

| Member                                                         | Return                              |
| -------------------------------------------------------------- | ----------------------------------- |
| `Change::new(impl IntoIterator<Item = Operation>)`             | Result&lt;Change&gt;                |
| `Change::noop()`                                               | Change                              |
| `operations()`, `is_noop()`                                    | &[Operation], bool                  |
| `encode()`, `Change::decode(&[u8])`                            | Vec&lt;u8&gt;, Result&lt;Change&gt; |
| `apply(&Value, &Change)`                                       | Result&lt;Value&gt;                 |
| `compose(&Value, &Change, &Change)`, `invert(&Value, &Change)` | Result&lt;Change&gt;                |
| `transform(&Value, &Change, &Change, Priority)`                | Result&lt;(Change, Change)&gt;      |

| Operation variant | Fields                                   |
| ----------------- | ---------------------------------------- |
| Insert            | `path: Path`, `value: Value`             |
| Delete            | `path: Path`                             |
| Set               | `path: Path`, `value: Value`             |
| ListMove          | `path: Path`, `from: usize`, `to: usize` |
| Text              | `path: Path`, `change: TextChange`       |
| Add               | `path: Path`, `delta: i64`               |
| RichText          | `path: Path`, `operations: Vec<RichOp>`  |

Insert's path ends with a List insertion index; Set writes a Map member, adding it if
missing, or replaces a List element or the root. ListMove's path names the List and `to` counts after removal. `Operation::path()` returns the path.
Priority is Left or Right. Transform returns `(left_after_right, right_after_left)`;
its inputs share a common base. Compose's second change applies after its first. See
[Changes](/docs/core/changes) and
[Change algebra](/docs/core/algebra).

TextOp variants are `Retain(usize)`, `Insert(String)` and `Delete(usize)`;
`TextChange::from_ops(impl IntoIterator<Item = TextOp>) -> Result<TextChange>`
constructs a text sequence. `ops() -> &[TextOp]` reads its canonical operations and
`is_empty() -> bool` checks for an empty stream.
RichOp variants are `Retain { len: usize, attrs: AttrPatch }`, `Insert(RichSpan)` and
`Delete(usize)`. All Rust text positions count Unicode scalars; embeds count one.

## Document

| Signature                                                                                  | Result                              |
| ------------------------------------------------------------------------------------------ | ----------------------------------- |
| `Document::create(value: Value)`                                                           | `Document`                          |
| `snapshot()`                                                                               | `Result<Value>`                     |
| `version()`                                                                                | `Result<u64>` local content version |
| `get(path: &[Segment])`                                                                    | `Result<Value>`                     |
| `kind(path: &[Segment])`                                                                   | `Result<&'static str>`              |
| `has(path: &[Segment])`                                                                    | `Result<bool>`                      |
| `edit(callback: impl FnOnce(&mut Transaction) -> Result<()>)`                              | `Result<Option<EditResult>>`        |
| `edit_group(group: Option<String>, callback: impl FnOnce(&mut Transaction) -> Result<()>)` | `Result<Option<EditResult>>`        |
| `apply(change: &Change)`                                                                   | `Result<Option<EditResult>>`        |
| `close()`                                                                                  | `Result<()>`                        |

Noop edits return None. Missing `get` or `kind` targets are errors. Runtime
reads fail after close; previously returned Values remain readable. Transaction errors
or unwinding abandon the edit. See [Transactions](/docs/editing/transactions).

## Transaction

| Signature                                                                        | Result          |
| -------------------------------------------------------------------------------- | --------------- |
| `snapshot()`                                                                     | `Result<Value>` |
| `get(path: &[Segment])`                                                          | `Result<Value>` |
| `set(path: &[Segment], value: Value)`                                            | `Result<()>`    |
| `delete(path: &[Segment])`                                                       | `Result<()>`    |
| `list_move(path: &[Segment], from: usize, to: usize)`                            | `Result<()>`    |
| `increment(path: &[Segment], delta: i64)`                                        | `Result<()>`    |
| `list_replace(path: &[Segment], index: usize, count: usize, values: Vec<Value>)` | `Result<()>`    |
| `text_replace(path: &[Segment], index: usize, count: usize, text: &str)`         | `Result<()>`    |
| `rich_text_edit(path: &[Segment], operations: Vec<RichOp>)`                      | `Result<()>`    |
| `utf16_to_scalar(path: &[Segment], position: usize)`                             | `Result<usize>` |
| `apply(change: &Change)`                                                         | `Result<()>`    |

Rust exposes sequence replacement methods directly on Transaction. A zero removal count
inserts; empty replacement content deletes. Text and RichText positions count Unicode
scalars; List positions count elements. `utf16_to_scalar` converts against working content
and rejects surrogate splits. Invalid ranges, missing parents and kind mismatches are
errors. See [Map and List editing](/docs/editing/maps-lists).

## EditResult and observation

| Field               | Type                                  |
| ------------------- | ------------------------------------- |
| `before`, `after`   | `Value`                               |
| `change`, `inverse` | `Change`                              |
| `version`           | `u64` local content version           |
| `origin`            | `Origin::{Local, Remote, Undo, Redo}` |

Rust returns edit results; subscription callbacks are a JavaScript facade feature.
See [Edit results](/docs/editing/results) for replay semantics.

## History

| Signature                                                              | Result                                  |
| ---------------------------------------------------------------------- | --------------------------------------- |
| `History::attach(document: &Document)`                                 | `Result<History>`; default capacity 100 |
| `History::attach_with_capacity(document: &Document, capacity: usize)`  | `Result<History>`                       |
| `History::restore(document: &Document, checkpoint: HistoryCheckpoint)` | `Result<History>`                       |
| `can_undo()`, `can_redo()`                                             | `Result<bool>`                          |
| `undo()`, `redo()`                                                     | `Result<Option<EditResult>>`            |
| `clear()`, `close()`                                                   | `Result<()>`                            |
| `checkpoint()`                                                         | `Result<HistoryCheckpoint>`             |

Attach reuses existing History. Restore requires its exact content basis; a mismatch
fails. See [History](/docs/history/) for grouping and collaborative undo.

## SyncSession

| Signature                                                                   | Result                                  |
| --------------------------------------------------------------------------- | --------------------------------------- |
| `SyncSession::create(client_id: impl Into<String>, snapshot: SyncSnapshot)` | `Result<SyncSession>`                   |
| `SyncSession::restore(checkpoint: SessionCheckpoint)`                       | `Result<SyncSession>`                   |
| `document()`                                                                | `Document`                              |
| `revision()`                                                                | `Result<u64>` confirmed server revision |
| `recovery_reason()`                                                         | `Result<Option<CollaError>>`            |
| `outbound()`                                                                | `Result<Option<Submission>>`            |
| `receive(message: &ServerMessage)`                                          | `Result<Option<EditResult>>`            |
| `checkpoint()`                                                              | `Result<SessionCheckpoint>`             |
| `close()`                                                                   | `Result<()>`                            |
| `is_closed()`                                                               | `bool`                                  |

Inspect `recovery_reason` when synchronization cannot continue. A matching Rejection
is received as a message and can set this reason without returning an error. A missing
Commit interval returns `missing_revision`. See [Retries](/docs/sync/retries) and
[Recovery](/docs/sync/recovery) for the respective workflows.

## Authority

| Signature                                                         | Result                               |
| ----------------------------------------------------------------- | ------------------------------------ |
| `Authority::create(document_id: impl Into<String>, value: Value)` | `Result<Authority>`                  |
| `Authority::restore(checkpoint: AuthorityCheckpoint)`             | `Result<Authority>`                  |
| `revision()`                                                      | `u64`                                |
| `snapshot()`                                                      | `SyncSnapshot`                       |
| `accept(submission: &Submission)`                                 | `Result<(Authority, ServerMessage)>` |
| `commits_since(revision: u64)`                                    | `Result<Vec<Commit>>`                |
| `compact(through_revision: u64)`                                  | `Result<Authority>`                  |
| `checkpoint()`                                                    | `AuthorityCheckpoint`                |

Authority is immutable. `commits_since` fails with `history_expired` below the retained
floor. Unlike JavaScript's ServerMessage wrappers, Rust returns Commit values; wrap
one in `ServerMessage::Commit(commit)` to pass it to `receive`. See
[Persistence](/docs/production/persistence#server-durability) before adopting returned state.

## Protocol objects and codecs

Value, Change, SyncSnapshot, Submission, ServerMessage, SessionCheckpoint,
HistoryCheckpoint and AuthorityCheckpoint provide `encode() -> Vec<u8>` and
`Type::decode(bytes: &[u8]) -> Result<Type>`. Runtime constructors and decoders create
controlled protocol objects; their fields are not public mutation APIs. Every Value and
Change is valid by construction, so decoding rejects invalid content and no separate
validation step exists. Checkpoint decoders check structure only; `restore` validates
a checkpoint's semantic consistency.

| Type         | Read methods                                                                                                         |
| ------------ | -------------------------------------------------------------------------------------------------------------------- |
| SyncSnapshot | `document_id() -> &str`, `revision() -> u64`, `value() -> &Value`                                                    |
| Submission   | `document_id() -> &str`, `client_id() -> &str`, `sequence() -> u64`, `base_revision() -> u64`, `change() -> &Change` |
| Commit       | `document_id() -> &str`, `client_id() -> &str`, `sequence() -> u64`, `revision() -> u64`, `change() -> &Change`      |
| Rejection    | `document_id() -> &str`, `client_id() -> &str`, `sequence() -> u64`, `reason() -> &CollaError`                       |

ServerMessage has `Commit(Commit)` and `Rejection(Rejection)` variants. Encode the
ServerMessage wrapper for transmission. See [Protocol and encoding](/reference/protocol)
for object fields, format and validation limits.

## Errors

CollaError exposes `code: ErrorCode`, `operation: String` and
`details: BTreeMap<String, String>`. Match the [stable codes](/reference/glossary#error-codes),
not message text. Rust `ErrorCode` variants use PascalCase; `code.as_str()` gives the
snake_case code used in the glossary. Decode rejects malformed or oversized bytes.
