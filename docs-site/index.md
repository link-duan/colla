---
layout: home
title: Colla
titleTemplate: Structured collaborative documents
hero:
  name: Colla
  text: Identity that moves with your content.
  tagline: Immutable documents, native Move and Ref, collaborative undo, and centralized sync. A synchronous JavaScript API powered by a compact Rust engine.
  actions:
    - theme: brand
      text: Get started
      link: /docs/getting-started/
    - theme: alt
      text: API reference
      link: /reference/javascript
features:
  - title: Stable identities
    details: Move any addressable subtree across parents. References and concurrent edits follow the same element.
  - title: Atomic editing
    details: One synchronous scope for lists, text, rich text and structure. Immutable snapshots remain usable after the document closes.
  - title: Collaboration included
    details: History, SyncSession and Authority coordinate undo, retries and offline recovery with explicit persistence boundaries.
  - title: Small by design
    details: Zero npm runtime dependencies, shared Rust codecs, and measured artifact and memory budgets.
---

```ts
import { Document, History, text, ref } from "colla-ot"
const doc = Document.create({ tasks: [{ title: text("Draft") }], done: [], selected: null })
const history = History.attach(doc)
const task = doc.idAt(["tasks", 0])
doc.edit(tx => {
  tx.text(["tasks", 0, "title"]).insert(5, " v2")
  tx.set(["selected"], ref(task))
  tx.move(task, { parent: ["done"], index: 0 })
})
history.undo()
```

Both language APIs use the same
base-aware `transform` and version-2 format. Transport, persistence storage,
authentication, presence and editor adapters belong to your application.


## Choose a reading path

- **Build your first editor:** [Getting started](/docs/getting-started/) → [Editing](/docs/editing/) → [History](/docs/history/).
- **Understand the model:** [Values](/docs/core/values) → [Element identity](/docs/core/identity) → [Changes and OT](/docs/core/changes).
- **Connect collaborators:** [Sync overview](/docs/sync/) → [Two-client example](/docs/examples/sync) → [Persistence](/docs/production/persistence).
