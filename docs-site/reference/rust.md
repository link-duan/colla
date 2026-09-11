# Rust API

Use `colla` for immutable content, transactional editing, History and synchronization.
Requires Rust 1.81 or newer. See [Getting started](/docs/getting-started/) for installation
and [Rust examples](/docs/examples/rust) for runnable programs.

Signatures below omit the receiver: reads use `&self`, Transaction mutations use
`&mut self`, and functions prefixed with a type name are associated functions.
`Result<T>` means `std::result::Result<T, CollaError>`. These tables are API lookup,
not standalone programs.

## Content types

| Type | Variants or fields |
| --- | --- |
| Body | `Null`, `Bool(bool)`, `Int(i64)`, `Float(f64)`, `String(String)`, `Text(String)`, `RichText(Vec<RichSpan>)`, `Ref(Ref)`, `List(Vec<Value>)`, `Map(BTreeMap<String, Value>)` |
| Ref | `target: ElementId` |
| RichSpan | `Text { text: String, attrs: Attrs }`, `Embed { value: Value, attrs: Attrs }` |
| Attr | `Bool(bool)`, `Int(i64)`, `Float(f64)`, `String(String)` |
| Attrs | `BTreeMap<String, Attr>` |
| AttrPatch | `BTreeMap<String, Option<Attr>>`; None removes an attribute |
| Path | `Vec<Segment>`; empty for the root |
| Segment | `Key(String)`, `Index(usize)` |
| Location | `Path(Path)`, `Id(ElementId)`; converts from Path or ElementId |

Value clones share immutable storage. Each owning node carries an ElementId.
Float values must be finite. See [Values and types](/docs/core/values) for the content model.

## Value methods

| Constructor or method | Return |
| --- | --- |
| `Value::new(body: Body)` | Result&lt;Value&gt; |
| `Value::with_allocator(body: Body, allocator: &mut IdAllocator)` | Result&lt;Value&gt; |
| `Value::null()`, `bool(bool)`, `int(i64)`, `reference(ElementId)` | Value |
| `Value::string(impl Into<String>)`, `text(impl Into<String>)`, `float(f64)`, `list(Vec<Value>)`, `map(impl IntoIterator<Item = (String, Value)>)`, `rich_text(Vec<RichSpan>)` | Result&lt;Value&gt; |
| `id()`, `body()`, `kind()` | ElementId, &Body, &'static str |
| `get(impl Into<Location>)`, `id_at(Path)` | Result&lt;Value&gt;, Result&lt;ElementId&gt; |
| `has(impl Into<Location>)` | bool |
| `find(ElementId)`, `path_of(ElementId)`, `resolve(Ref)` | Option&lt;&Value&gt;, Option&lt;Path&gt;, Option&lt;Value&gt; |
| `references_to(ElementId)` | Vec&lt;ElementId&gt; |
| `content_equals(&Value)` | bool |
| `copied()` | Result&lt;Value&gt; with fresh IDs |
| `validate()` | Result&lt;()&gt; |
| `encode()`, `Value::decode(&[u8])` | Vec&lt;u8&gt;, Result&lt;Value&gt; |

Unlike JavaScript get, Rust get reports missing values as an error. `find` looks up an
ID and returns None if absent. Ref resolution is explicit and advances one hop.

## Change methods and types

| Member | Return |
| --- | --- |
| `Change::new(impl IntoIterator<Item = Operation>)` | Result&lt;Change&gt; |
| `Change::noop()` | Change |
| `operations()`, `is_noop()` | &[Operation], bool |
| `encode()`, `Change::decode(&[u8])` | Vec&lt;u8&gt;, Result&lt;Change&gt; |
| `apply(&Value, &Change)` | Result&lt;Value&gt; |
| `compose(&Value, &Change, &Change)`, `invert(&Value, &Change)` | Result&lt;Change&gt; |
| `transform(&Value, &Change, &Change, Priority)` | Result&lt;(Change, Change)&gt; |

| Operation variant | Fields |
| --- | --- |
| Insert | `destination: Destination`, `value: Value` |
| Delete | `target: ElementId` |
| Set | `target: ElementId`, `value: Value` |
| Move | `target: ElementId`, `destination: Destination` |
| Text | `target: ElementId`, `change: TextChange` |
| Add | `target: ElementId`, `delta: i64` |
| RichText | `target: ElementId`, `operations: Vec<RichOp>` |

Destination contains `parent: ElementId` and `slot: Segment`. Priority is Left or Right.
Transform returns `(left_after_right, right_after_left)`; its inputs share a common base.
Compose's second change applies after its first. See [Changes and OT algebra](/docs/core/changes).

TextOp variants are `Retain(usize)`, `Insert(String)` and `Delete(usize)`;
`TextChange::from_ops(impl IntoIterator<Item = TextOp>) -> Result<TextChange>`
constructs a text sequence. `ops() -> &[TextOp]` reads its canonical operations and
`is_empty() -> bool` checks for an empty stream.
RichOp variants are `Retain { len: usize, attrs: AttrPatch }`, `Insert(RichSpan)` and
`Delete(usize)`. All Rust text positions count Unicode scalars; embeds count one.

## Document

| Signature | Result |
| --- | --- |
| `Document::create(value: Value)` | `Result<Document>` |
| `snapshot()` | `Result<Value>` |
| `version()` | `Result<u64>` local content version |
| `get(location: impl Into<Location>)` | `Result<Value>` |
| `kind(location: impl Into<Location>)` | `Result<&'static str>` |
| `has(location: impl Into<Location>)` | `Result<bool>` |
| `id_at(path: Path)` | `Result<ElementId>` |
| `path_of(id: ElementId)` | `Result<Option<Path>>` |
| `resolve(reference: Ref)` | `Result<Option<Value>>` |
| `references_to(id: ElementId)` | `Result<Vec<ElementId>>` |
| `edit(callback: impl FnOnce(&mut Transaction) -> Result<()>)` | `Result<Option<EditResult>>` |
| `edit_group(group: Option<String>, callback: impl FnOnce(&mut Transaction) -> Result<()>)` | `Result<Option<EditResult>>` |
| `apply(change: &Change)` | `Result<Option<EditResult>>` |
| `close()` | `Result<()>` |

Noop edits return None. Missing `get`, `kind` or `id_at` targets are errors. Runtime
reads fail after close; previously returned Values remain readable. Transaction errors
or unwinding abandon the edit. See [Transactions](/docs/editing/transactions).

## Transaction

| Signature | Result |
| --- | --- |
| `snapshot()` | `Result<Value>` |
| `get(location: impl Into<Location>)` | `Result<Value>` |
| `set(location: impl Into<Location>, value: Value)` | `Result<()>` |
| `delete(location: impl Into<Location>)` | `Result<()>` |
| `move_to(source: impl Into<Location>, parent: impl Into<Location>, slot: Segment)` | `Result<()>` |
| `copy(source: impl Into<Location>, parent: impl Into<Location>, slot: Segment)` | `Result<ElementId>` new root ID |
| `increment(target: impl Into<Location>, delta: i64)` | `Result<()>` |
| `list_replace(target: impl Into<Location>, index: usize, count: usize, values: Vec<Value>)` | `Result<()>` |
| `text_replace(target: impl Into<Location>, index: usize, count: usize, text: &str)` | `Result<()>` |
| `rich_text_edit(target: impl Into<Location>, operations: Vec<RichOp>)` | `Result<()>` |
| `utf16_to_scalar(target: impl Into<Location>, position: usize)` | `Result<usize>` |
| `apply(change: &Change)` | `Result<()>` |

Rust exposes sequence replacement methods directly on Transaction. A zero removal count
inserts; empty replacement content deletes. Text and RichText positions count Unicode
scalars; List positions count elements. `utf16_to_scalar` converts against working content
and rejects surrogate splits. Invalid ranges, missing parents, occupied Move/Copy Map
slots and kind mismatches are errors. See [Move, Copy and Set](/docs/core/move-copy-set).

## EditResult and observation

| Field | Type |
| --- | --- |
| `before`, `after` | `Value` |
| `change`, `inverse` | `Change` |
| `edit_steps` | `Vec<Operation>` |
| `version` | `u64` local content version |
| `origin` | `Origin::{Local, Remote, Undo, Redo}` |

Rust returns edit results; subscription callbacks are a JavaScript facade feature.
See [Edit results and steps](/docs/editing/results) for replay semantics.

## History

| Signature | Result |
| --- | --- |
| `History::attach(document: &Document)` | `Result<History>`; default capacity 100 |
| `History::attach_with_capacity(document: &Document, capacity: usize)` | `Result<History>` |
| `History::restore(document: &Document, checkpoint: HistoryCheckpoint)` | `Result<History>` |
| `can_undo()`, `can_redo()` | `Result<bool>` |
| `undo()`, `redo()` | `Result<Option<EditResult>>` |
| `clear()`, `close()` | `Result<()>` |
| `checkpoint()` | `Result<HistoryCheckpoint>` |

Attach reuses existing History. Restore requires its exact content basis; a mismatch
fails. See [History](/docs/history/) for grouping and collaborative undo.

## SyncSession

| Signature | Result |
| --- | --- |
| `SyncSession::create(client_id: impl Into<String>, snapshot: SyncSnapshot)` | `Result<SyncSession>` |
| `SyncSession::restore(checkpoint: SessionCheckpoint)` | `Result<SyncSession>` |
| `document()` | `Document` |
| `revision()` | `Result<u64>` confirmed server revision |
| `recovery_reason()` | `Result<Option<CollaError>>` |
| `outbound()` | `Result<Option<Submission>>` |
| `receive(message: &ServerMessage)` | `Result<Option<EditResult>>` |
| `checkpoint()` | `Result<SessionCheckpoint>` |
| `close()` | `Result<()>` |
| `is_closed()` | `bool` |

Inspect `recovery_reason` when synchronization cannot continue. A matching Rejection
is received as a message and can set this reason without returning an error. A missing
Commit interval returns `missing_revision`. See [Retries](/docs/sync/retries) and
[Recovery](/docs/sync/recovery) for the respective workflows.

## Authority

| Signature | Result |
| --- | --- |
| `Authority::create(document_id: impl Into<String>, value: Value)` | `Result<Authority>` |
| `Authority::restore(checkpoint: AuthorityCheckpoint)` | `Result<Authority>` |
| `revision()` | `u64` |
| `snapshot()` | `SyncSnapshot` |
| `accept(submission: &Submission)` | `Result<(Authority, ServerMessage)>` |
| `commits_since(revision: u64)` | `Result<Vec<Commit>>` |
| `compact(through_revision: u64)` | `Result<Authority>` |
| `checkpoint()` | `AuthorityCheckpoint` |

Authority is immutable. `commits_since` fails with `history_expired` below the retained
floor. Unlike JavaScript's ServerMessage wrappers, Rust returns Commit values; wrap
one in `ServerMessage::Commit(commit)` to pass it to `receive`. See
[Persistence](/docs/production/persistence#server-durability) before adopting returned state.

## Protocol objects and codecs

Value, Change, SyncSnapshot, Submission, ServerMessage, SessionCheckpoint,
HistoryCheckpoint and AuthorityCheckpoint provide `encode() -> Vec<u8>` and
`Type::decode(bytes: &[u8]) -> Result<Type>`. Runtime constructors and decoders create
controlled protocol objects; their fields are not public mutation APIs.

| Type | Read methods |
| --- | --- |
| SyncSnapshot | `document_id() -> &str`, `revision() -> u64`, `value() -> &Value` |
| Submission | `document_id() -> &str`, `client_id() -> &str`, `sequence() -> u64`, `base_revision() -> u64`, `change() -> &Change` |
| Commit | `document_id() -> &str`, `client_id() -> &str`, `sequence() -> u64`, `revision() -> u64`, `change() -> &Change` |
| Rejection | `document_id() -> &str`, `client_id() -> &str`, `sequence() -> u64`, `reason() -> &CollaError` |

ServerMessage has `Commit(Commit)` and `Rejection(Rejection)` variants. Encode the
ServerMessage wrapper for transmission. See [Protocol and encoding](/reference/protocol)
for object fields, format and validation limits.

## Identity and errors

ElementId implements Display and FromStr; `namespace(self) -> [u8; 16]` and
`sequence(self) -> u64` expose its parts. IdAllocator provides `random() -> Result<Self>`,
`deterministic(namespace: [u8; 16]) -> Self`, `allocate(&mut self) -> Result<ElementId>`
and `scope<T>(&mut self, callback: impl FnOnce() -> T) -> T`. Deterministic namespaces
must be distinct between tests; normal construction uses secure random namespaces.

CollaError exposes `code: ErrorCode`, `operation: String` and
`details: BTreeMap<String, String>`. When present, `details["elementId"]` contains
the diagnostic element identity. Match the [stable codes](/reference/glossary#error-codes),
not message text. Rust `ErrorCode` variants use PascalCase; `code.as_str()` gives the
snake_case code used in the glossary. Decode rejects malformed or oversized bytes.
