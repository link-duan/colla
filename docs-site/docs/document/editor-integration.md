---
title: Editor integration
description: Apply Document change events to editors without echo loops or coordinate errors.
---

# Editor integration

Document events are editor projections. They are deliberately separate from
Core Change input and from Snapshot/Update wire bytes.

```ts
// `document`, `editor`, and `report` are application-owned objects.
const unsubscribe = document.subscribe({
  onChange: event => {
    if (event.origin === 'remote') {
      editor.applyEditSteps(event.editSteps)
    }
  },
  onError: ({ error }) => report(error),
})
```

## Avoiding echo loops

The local editor usually already rendered the keystroke that produced the local
mutation. Send the returned `Update` (`update.bytes`) to transport, but do not
apply the local event back to the same editor. Remote events must be applied once, in arrival order:

| Event origin | Editor action | Network action |
| --- | --- | --- |
| `local` | Usually no second render | Enqueue and send `update.bytes` returned by `transact()`. |
| `remote` | Apply `event.editSteps` | No echo back to the server. |

Keep editor transactions, network acknowledgements, and adapter retry state in
separate controllers. Call `unsubscribe()` before unmounting the editor.

## Edit-step shape

`editSteps` is a recursively frozen array. Each step has a `path` and one of
`replace`, `int`, `map`, `list`, `text`, or `richtext` operations. Map
modifies recurse into child steps; lists retain, insert, delete, or modify
elements; Text and RichText expose sequence operations suitable for an adapter.

The projection requires the previous visible Value, which Document supplies
internally. It reports editor positions in UTF-16 code units, while Core Change
lengths count Unicode scalars. A RichText embed counts as one logical unit.

## Coordinates and teardown

Convert selections against the Snapshot-relative Value that created them. Do
not persist list indexes as IDs, and never split a UTF-16 surrogate pair. See
[Coordinates](/docs/core/coordinates) for explicit conversion helpers.

Listener state commits before notification. If an adapter throws, other
listeners still run and the error listener receives the failure; the committed
Document state is not rolled back.

Next: [Events and lifecycle](/docs/document/events-lifecycle).
