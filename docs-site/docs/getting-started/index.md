# Getting started

Colla is an Operational Transformation library for structured collaborative documents.
It provides immutable content, stable element identities, atomic editing, undo/redo and
centralized synchronization. The Rust engine is exposed to JavaScript through the
synchronous `colla-ot` package, with no npm runtime dependencies.

## Install

The JavaScript package requires Node.js 22 or newer and supports modern browsers,
Dedicated Workers and Shared Workers. All environments use the same ESM import;
there is no public initialization or Wasm disposal step.

The first official release has not been published yet. The commands below apply once
version 0.4 is available on npm and crates.io.

```sh
npm install colla-ot@^0.4.0
```

For Rust, add `colla = "0.4"` to `[dependencies]`; the minimum Rust version is 1.81.

## Make your first edit

Create a Document with one collaborative text field, append a suffix, and read the
updated content. The edit is visible as soon as the callback returns.

<<< ../../examples/first-edit.ts

`text()` creates collaborative text; an ordinary string is atomic. The callback receives
a scoped Transaction and must finish synchronously. Use `close()` when editing is finished.

## Add collaboration

Use a standalone Document for local editing. Attach History to add undo and redo.
For synchronized editing, create a SyncSession from an Authority's SyncSnapshot and edit
`session.document`. A Value is immutable content, a Document owns editable state, and a
SyncSession coordinates server confirmation.

Your application supplies transport, persistence, authentication and editor integration.
Colla does not open sockets or write databases. Presence and cross-document synchronization
are also application responsibilities.

## Next steps

- Learn [Values and types](/docs/core/values) to choose your content model.
- Read [Editing](/docs/editing/) and [History](/docs/history/) to build an editor.
- Follow the [two-client example](/docs/examples/sync) to connect collaborators.
- Review [Persistence](/docs/production/persistence) before deploying your application.
