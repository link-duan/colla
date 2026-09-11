# colla-ot

Synchronous collaborative documents powered by Rust and WebAssembly. Provides stable
identity, atomic editing, collaborative undo/redo and centralized synchronization.
Node.js 22+, modern browsers and workers use the same ESM import. There are no npm
runtime dependencies or public Wasm initialization steps.

## Install

The first official release has not been published yet. Once 0.4 is available:

```sh
npm install colla-ot@^0.4.0
```

## First edit

```ts
import { Document, text } from 'colla-ot'

const doc = Document.create({ title: text('Draft') })
doc.edit(tx => tx.text(['title']).insert(5, ' v2'))
console.log('Updated title:', doc.get(['title'])?.toJS()) // Text containing Draft v2
doc.close()
```

`text()` opts into collaborative text editing. Ordinary strings are replaced as a whole.
The transaction callback runs synchronously; close the runtime when editing is finished.

## Documentation

- [Getting started](https://link-duan.github.io/colla/docs/getting-started/)
- [JavaScript API](https://link-duan.github.io/colla/reference/javascript)
- [Two-client synchronization](https://link-duan.github.io/colla/docs/examples/sync)
- [Persistence and restart](https://link-duan.github.io/colla/docs/production/persistence)
