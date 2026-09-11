# Transactions and editors

`doc.edit(callback, { group? })` runs a synchronous scoped Transaction. All content,
version, history and pending effects are computed before commit. Throwing or failing
validation abandons the entire edit.

## Choose an editor

| Surface | Methods |
| --- | --- |
| Transaction | set, delete, move, copy, increment, apply |
| `tx.list(location)` | insert, delete, replace |
| `tx.text(location)` | insert, delete, replace |
| `tx.richText(location)` | insertText, insertEmbed, delete, replace, format |

List insertion takes an array of Input values. Replace takes an index, removal count
and replacement values. Text replacement takes a string; RichText replacement takes
spans. `increment` accepts bigint and requires an Int target with no i64 overflow.

Start with [Map and List editing](./maps-lists), [Text](/docs/core/text) or
[RichText](/docs/core/richtext) for operation examples.

## Scope restrictions

Do not await, return a Promise/thenable, nest edits, receive remote messages, or close
the Document inside a transaction. Fetch data before entering the callback. Transactions
and derived editors become invalid when the callback exits, including read access.
Retain IDs or immutable Values between edits, not editors.

## Results and failures

The return value is an EditResult or null for a normalized Noop; callback return data
is not the edit result. Missing targets and invalid ranges fail rather than silently
skipping input. `doc.apply(change)` commits an already constructed Change through the
Document; `tx.apply(change)` includes it in a larger atomic transaction.

A valid increment is rolled back if a later text operation fails in the same callback.
Start with the [first-edit example](/docs/getting-started/#make-your-first-edit), and use
[grouping](/docs/history/grouping) when several transactions should undo as one user action.
