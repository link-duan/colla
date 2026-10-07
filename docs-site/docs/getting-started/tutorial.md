# Tutorial: a shared task list

This tutorial builds a grocery list step by step: model the content, edit it, react to
changes, add undo, and finally let two users edit it at the same time. All snippets are
parts of one program, [`examples/tutorial.ts`](https://github.com/link-duan/colla/blob/master/docs-site/examples/tutorial.ts),
and run in order. The block after each snippet shows what it prints.

## 1. Model the content

A Document holds editable content. Create it from ordinary JavaScript values:

<<< ../../examples/tutorial.ts#model

Choose a type for each field by how users edit it. The list name and item names are
`text(...)` so that several users can type into them at once; a plain string could only
be replaced as a whole. `done` is a boolean, and `items` is a List so that items can be
inserted, deleted and reordered. [Values](/docs/core/values#choose-the-right-kind)
explains the trade-offs.

You read content with a [Path](/docs/core/paths): Map keys and List indexes from the
root. Reads return immutable Values; `toJS()` turns them into plain JavaScript.

```
First item: Text { type: 'text', value: 'Milk' }
```

## 2. Edit in a transaction

All changes happen inside `doc.edit`. The callback receives a Transaction with editors
for Lists, Text and other values:

<<< ../../examples/tutorial.ts#edit

```
Title: Text { type: 'text', value: 'Groceries for Sunday' }
Milk done: true
```

The three edits commit together when the callback returns. Each call sees the result of
the calls before it, so the inserted `Eggs` item is already at index 1 inside the
callback. Text offsets count UTF-16 code units, like JavaScript strings and browser
selections.

## 3. Failed edits change nothing

If any call in the callback fails, the whole transaction is abandoned:

<<< ../../examples/tutorial.ts#rollback

```
Edit failed: out_of_bounds
Eggs done: false
```

The first `set` was valid, but it is discarded together with the failing `delete`.
Errors are `CollaError`s with a stable `code` you can branch on. The callback must also
be synchronous: fetch data before calling `doc.edit`, never await inside it.

## 4. React to changes

A view subscribes to the Document and re-renders after each commit:

<<< ../../examples/tutorial.ts#subscribe

Each event carries the content `before` and `after` the commit, the Change that was
applied, and an `origin`: `local`, `remote`, `undo` or `redo`. Listeners run
synchronously after the commit and may read, but not edit. See
[Subscriptions and events](/docs/editing/events).

## 5. Add undo

Attach a History to record local edits:

<<< ../../examples/tutorial.ts#undo

```
Committed: local version 2n
First item after delete: Text { type: 'text', value: 'Eggs' }
Committed: undo version 3n
First item after undo: Text { type: 'text', value: 'Milk' }
```

Undo is itself an edit: it commits new content, advances the version and notifies
subscribers with origin `undo`. Each `doc.edit` call is one undo step by default;
[Grouping](/docs/history/grouping) merges several into one.

## 6. Edit together

To share the list, the server creates an Authority from the content, and each user
starts a SyncSession from the Authority's snapshot. Each session owns a Document, which
you edit exactly like before:

<<< ../../examples/tutorial.ts#sync

```
Alice, first item: Text { type: 'text', value: 'Bread' }
Alice, second item: Text { type: 'text', value: 'Milk (oat)' }
Bob, second item: Text { type: 'text', value: 'Milk (oat)' }
```

Both edits were made against the same snapshot: Alice renamed the item at index 0 while
Bob inserted a new item at index 0. Each user saw their own edit immediately. When the
commits arrived, Colla transformed them so that Alice's rename still landed on `Milk`,
now at index 1, and both copies converged.

`deliver` stands in for your network. In a real application, the client sends
`submission.encode()` to the server, the server calls `authority.accept`, stores the new
Authority, and broadcasts the commit's encoded bytes to every client. The
[synchronization overview](/docs/sync/) walks through that flow.

## 7. Clean up

Close what you created when the view or connection goes away:

<<< ../../examples/tutorial.ts#close

Values and snapshots you read earlier stay usable after closing.

## Where to go next

- **Model your content:** [Core concepts](/docs/core/), then [Text and RichText](/docs/core/text).
- **Build the editor:** [Editing](/docs/editing/), [Editor integration](/docs/editing/editor-integration) and [History](/docs/history/).
- **Connect real clients:** [Synchronization overview](/docs/sync/), the [two-client example](/docs/examples/sync), and [Retries](/docs/sync/retries).
- **Ship it:** [Persistence](/docs/production/persistence), [Transport](/docs/production/transport) and [Errors and limits](/docs/production/errors-limits).
