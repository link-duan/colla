---
layout: home
title: Colla
titleTemplate: Structured collaborative documents
hero:
  name: Colla
  text: Structured content, edited together.
  tagline: Immutable documents, path-based OT, collaborative undo, and centralized sync. A synchronous JavaScript API powered by a compact Rust engine.
  actions:
    - theme: brand
      text: Get started
      link: /docs/getting-started/
    - theme: alt
      text: API reference
      link: /reference/javascript
features:
  - title: Plain content
    details: Values carry no hidden identity. Paths address content, and concurrent edits follow elements through insertions, deletions and List moves.
  - title: Atomic editing
    details: One synchronous scope for lists, text, rich text and structure. Immutable snapshots remain usable after the document closes.
  - title: Collaboration included
    details: History, SyncSession and Authority coordinate undo, retries and offline recovery with explicit persistence boundaries.
  - title: Small by design
    details: Zero npm runtime dependencies, shared Rust codecs, and measured artifact and memory budgets.
---

```ts
import { Document, History, text } from 'colla-ot'
const doc = Document.create({ tasks: [{ title: text('Draft') }, { title: text('Review') }] })
const history = History.attach(doc)
doc.edit(tx => {
  tx.text(['tasks', 0, 'title']).insert(5, ' updated')
  tx.list(['tasks']).move(0, 1)
})
history.undo()
```

Both language APIs use the same
base-aware `transform` and binary format. Transport, persistence storage,
authentication, presence and editor adapters belong to your application.

## Choose a reading path

- **Build your first editor:** [Getting started](/docs/getting-started/) → [Editing](/docs/editing/) → [History](/docs/history/).
- **Understand the model:** [Values](/docs/core/values) → [Paths](/docs/core/paths) → [Changes and OT](/docs/core/changes).
- **Connect collaborators:** [Sync overview](/docs/sync/) → [Two-client example](/docs/examples/sync) → [Persistence](/docs/production/persistence).
