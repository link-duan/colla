# Installation

The first official release has not been published yet. The commands below apply once
version 0.4 is available on npm and crates.io.

## JavaScript

```sh
npm install colla-ot@^0.4.0
```

The package runs on Node.js 22 or newer, modern browsers, Dedicated Workers and Shared
Workers. Every environment uses the same ESM import:

```ts
import { Document, text } from 'colla-ot'
```

The Rust engine is bundled as WebAssembly and loaded by the import itself. There is no
initialization call, the API is synchronous, and nothing needs to be disposed besides
the objects you `close()`. The package has no npm runtime dependencies and ships its
own TypeScript declarations.

## Rust

Add the crate to `Cargo.toml`:

```toml
[dependencies]
colla = "0.4"
```

The minimum supported Rust version is 1.81. The crate exposes the same model, editing,
history and synchronization APIs; see the [Rust example](/docs/examples/rust) and the
[Rust API reference](/reference/rust).

## Mixing languages

JavaScript and Rust use the same binary format for every encoded object, so a
JavaScript client can synchronize with a Rust server, and either side can read content
the other stored. See [Protocol and encoding](/reference/protocol).

Continue with the [tutorial](./tutorial).
