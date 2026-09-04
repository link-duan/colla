# Example: editor integration

This example shows the direction of data flow between an editor and a Document.

```ts
import { Document, text } from 'colla-ot'

const document = Document.fromJS(text('Draft'))

const unsubscribe = document.subscribe({
  onChange: event => {
    if (event.origin === 'remote') {
      editor.applyEditSteps(event.editSteps)
    }
  },
  onError: ({ error }) => {
    telemetry.record('colla.listener_error', { code: error.code })
  },
})

editor.onInput(input => {
  const update = document.transact(tx => {
    tx.text([], t => t.retain(input.position).insert(input.text))
  })
  outbound.enqueue(update.bytes)
})

unsubscribe()
document.dispose()
```

The editor remains responsible for converting its UTF-16 offsets to Colla scalar positions.
The Document remains responsible for validation, visible state, rebasing, and event ordering.
Do not feed a local event back into the same editor transaction.
Apply remote steps once and associate them with the render generation that produced them.
Re-resolve list paths after sequence edits because indexes are Snapshot-relative.

## Next

Read [events and editor integration](../document/editor-integration), then [production testing](../production/testing).
