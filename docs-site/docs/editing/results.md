# Edit results and steps

Every non-Noop Document commit produces an immutable EditResult. The same structure is
delivered as an EditEvent to subscribers, after the commit has succeeded.

## Result fields

| Field | Meaning |
| --- | --- |
| before / after | Immutable Values at the commit boundaries |
| change / inverse | Applied Change and its inverse |
| editSteps | Replayable ordered operations with scalar sequence positions |
| version | bigint local content version |
| origin | local, remote, undo or redo |

The inverse restores the exact previous content and identities when applied to after.
It is not automatically valid against arbitrary later content; collaborative History
manages rebasing inverses for undo across remote edits.

## Replay rather than reinterpret

`apply(result.before, Change.create(result.editSteps))` reproduces `result.after`.
Move steps retain source IDs and destination semantics. They are not an editor-specific
UI delta and their text positions are not UTF-16. Process each step against its current
working Value if translating positions into an external editor.

## Lifetime and observation

Values, Changes and results remain readable after the runtime closes. An edit returning
null produces no result and no event. A server confirmation that does not change visible
content is observed through the SyncSession state channel instead of a Document event.

See [Subscriptions](./events) for dispatch restrictions and the
[editor adapter example](/docs/examples/editor) for checked replay of Edit Steps.
