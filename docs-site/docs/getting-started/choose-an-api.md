# Choose an API

<p class="lead">Pick the smallest layer that owns the state you need.</p>

| You are building… | Start with | Why |
| --- | --- | --- |
| A browser editor or JavaScript application | `colla-ot` | Owns visible content, revisions, events, local Updates, and Snapshots. |
| JS-side immutable operations or a custom state layer | `colla-ot/core` | Exposes Values, Changes, codecs, and OT algebra without Document state. |
| A Rust service, native application, or protocol adapter | `colla` | Reference implementation with typed constructors and typed errors. |
| A complete collaboration product | Colla plus your services | Transport, server ordering, auth, history, presence, and editor UI remain yours. |

## Package root: `colla-ot`

Use the root entry when your application needs one current document and a
pending outbound queue:

```ts
import { Document, Snapshot, Update } from 'colla-ot'
```

The `Document` wrapper handles optimistic local edits, ordered remote Updates,
acknowledgements, rebasing, and typed change/error events.

## Core entry: `colla-ot/core`

Use the Core entry when a function should be pure and snapshot-oriented:

```ts
import {
  Change,
  ValueHandle,
  apply,
  compose,
  invert,
  transformPair,
} from 'colla-ot/core'
```

Core operations do not know about users, revisions, transport, or persistence.
That makes them useful for reducers, tests, server adapters, and custom
document containers.

## Rust crate: `colla`

The Rust crate exposes the same semantic model with native ownership and typed
`Result` errors. Use it directly for native or server-side code, or as the
reference when implementing another language binding.

## A useful rule

Use `Document` at the application boundary. Use Core Values and Changes inside
your domain logic. Serialize a `Snapshot` for a checkpoint or an `Update` for
the application transport; do not serialize a JavaScript `Document` object.

Next: [Install and make a first edit](/docs/getting-started/install) or read
[Document synchronization](/docs/examples/javascript-document).
