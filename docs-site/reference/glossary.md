# Glossary and errors

The terms below follow the project domain model. Use them consistently when describing
content, editing intent and synchronization state.

## Domain terms

### Value

An immutable owning content tree without identity; equality is structural. It is not an editable Document.

### Snapshot

A Value observed at one moment. Its paths see only that content; it is not necessarily a confirmed SyncSnapshot.

### Path

An array of Map keys and List indexes interpreted against one content state. It is the
only way to address content and changes as content is edited.

### Map and List

Map owns children under string keys; List owns an ordered sequence of children.

### ListMove

Native move of one element within the same List. Concurrent edits inside the element follow it.

### Change

An ordered immutable operation sequence. Each path and sequence position refers to the content after the preceding operations; positions are scalar, not UI UTF-16 offsets.

### Document

Runtime owning visible content and local editing state. It is not the immutable Value itself.

### Transaction

Synchronous scoped working state that commits editing effects atomically.

### Local version

The local sequence of visible content commits, distinct from server revision.

### History

Grouped local undo/redo intent rebased across remote edits. It is not the Authority log.

### Server revision

Authority-assigned order of formal Commits, including transformed Noops. It tracks
confirmation separately from a Document’s local content version.

### SyncSnapshot

Confirmed Value plus document identity and server revision.

### SyncSession

Client state coordinating confirmed content, original in-flight request, rebased pending work and buffer.

### Submission

A request identified by document, client and sequence, carrying a base revision and immutable Change.

### Commit

A formally ordered server submission with a revision. The sender’s Commit also confirms its request.

### Rejection

A server response identifying the rejected request and its error. A matching rejection
can put the submitting session into recovery-required without discarding its work.

### Authority

Immutable server collaboration state owning commit order, retained history and request receipts.

### SessionCheckpoint

Complete client restoration state, including request identity, visible content and enabled History.

### HistoryCheckpoint

Undo/redo stacks, grouping and capacity tied to an exact content basis.

### AuthorityCheckpoint

Server restoration state containing the history floor, retained commits and request receipts.

### Recovery-required

A state preserving local work while outbound synchronization is halted pending explicit reconciliation.

### Priority

Consistent left/right intent arbitration used in transformation, not timestamp ordering.

### Canonical form

Deterministic normalized representation of one controlled object; not equivalence of all changes with the same effect.

### Golden fixtures

Shared fixed inputs, outputs and canonical bytes used as regression evidence, not an independent codec implementation.

### String

Atomic string replaced as a whole, distinct from collaborative Text.

### Text

Collaborative Unicode sequence. JavaScript high-level editing uses UTF-16; low-level operations use scalars.

### Int

Signed i64 with checked addition, represented by bigint in JavaScript.

### Float

Finite f64 represented by JavaScript number; it does not support increment.

### RichText

A sequence of attributed text and atomic embeds, independent of any HTML or editor-specific delta schema.

### Embed

One atomic Value occupying one RichText sequence position.

### Change position

Unicode scalar coordinate in low-level Text/RichText changes; an embed counts one.

### Editing position

UTF-16 coordinate in high-level JavaScript text editing, interpreted against each working step.

### CollaError

A public failure with stable code, operation and diagnostic details; message text is not a matching contract.

## Error codes

| Code                     | Meaning                                                                            |
| ------------------------ | ---------------------------------------------------------------------------------- |
| `invalid_argument`       | Unsupported argument shape or operation request.                                   |
| `invalid_value`          | Invalid content, such as malformed Unicode or nonfinite numeric input.             |
| `invalid_encoding`       | Malformed, noncanonical, wrong-version or wrong-type bytes.                        |
| `invalid_state`          | Closed runtime, expired editor, invalid restoration basis or forbidden reentrancy. |
| `limit_exceeded`         | Input exceeds a resource bound.                                                    |
| `type_mismatch`          | Operation does not match the target kind.                                          |
| `missing_key`            | Required target is absent.                                                         |
| `out_of_bounds`          | Invalid sequence index or removal range.                                           |
| `integer_overflow`       | Checked i64 arithmetic overflow.                                                   |
| `incompatible_change`    | Change cannot be interpreted against the supplied basis.                           |
| `invalid_utf16_boundary` | High-level text offset splits a surrogate pair.                                    |
| `missing_revision`       | Receive requires an earlier revision interval; content remains unchanged.          |
| `history_expired`        | Required rebase history is no longer retained.                                     |

Match code rather than human-readable reason text. JavaScript details is an immutable
string-valued record; elementId is optional. Listener exceptions occur after commit,
while local transaction errors abort all edits in the transaction. For remote errors,
inspect SyncSession.state to distinguish gap catch-up from recovery-required.

See [Errors and resource limits](/docs/production/errors-limits) and
[Recovery](/docs/sync/recovery) for application handling.
