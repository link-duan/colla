# Getting started

<p class="eyebrow">Start here</p>
<p class="lead">Colla gives your application a predictable document state machine and the OT primitives behind it.</p>

This page answers two questions: what Colla provides and how to make the first edit.

## What Colla provides

Colla combines immutable content and OT operations with a running `Document`
state machine in one JavaScript package.

All JavaScript capabilities are exported from `colla-ot`. The Rust `colla`
crate is the reference implementation for the data model, algebra, and
canonical bytes.

## Choose your workflow

The same package supports document state, immutable content operations, and
protocol integration. Start with the workflow that matches your application.

<div class="path-grid">
  <a class="path-card" href="./examples/javascript-document">
    <div class="path-card-body">
      <span class="path-card-tag">Browser &amp; Node</span>
      <strong class="path-card-title">Build a JavaScript app</strong>
      <p class="path-card-desc">Use a Document at the application boundary and connect your own transport.</p>
    </div>
    <span class="path-card-arrow" aria-hidden="true">→</span>
  </a>
  <a class="path-card" href="./examples/rust">
    <div class="path-card-body">
      <span class="path-card-tag">Native &amp; Server</span>
      <strong class="path-card-title">Build with Rust</strong>
      <p class="path-card-desc">Use typed constructors, native ownership, and reference OT algebra.</p>
    </div>
    <span class="path-card-arrow" aria-hidden="true">→</span>
  </a>
  <a class="path-card" href="./ot/protocol-boundaries">
    <div class="path-card-body">
      <span class="path-card-tag">Architecture</span>
      <strong class="path-card-title">Implement a protocol</strong>
      <p class="path-card-desc">Understand canonical wire bodies and Snapshot/Update envelopes.</p>
    </div>
    <span class="path-card-arrow" aria-hidden="true">→</span>
  </a>
</div>

## Install

For a copy-paste setup and first-run troubleshooting, see [Install and make a first edit](/docs/getting-started/install).

### JavaScript

```sh
pnpm add colla-ot
```

`colla-ot` is ESM-only and supports Node.js 22+, Vite 5+, and Rollup 4+.
Browser and Node entry points initialize the same WebAssembly core
synchronously.

### Rust

```toml
[dependencies]
colla = "0.3"
```

The crate supports Rust 1.81 or newer.

## Make your first edit

```ts
import { Document, Change, text } from 'colla-ot'

const document = Document.fromJS(text('Draft'))

const change = Change.build(change => {
  change.text(text => text.retain(5).insert(' v2'))
})

const update = document.applyLocal(change)
console.log(document.value().toJS())
console.log(update.updateId)
```

`applyLocal()` updates visible content immediately. The returned `Update` is
the unit your application sends to its server. Acknowledgement happens after
server acceptance:

```ts
document.ack(update.updateId)
```

## The boundary to remember

<div class="boundary-grid">
  <div class="boundary owns"><h3>Colla owns</h3><ul><li>Core Values and Changes</li><li>OT algebra and deterministic rebasing</li><li>Document state and events</li><li>Snapshot and Update envelopes</li></ul></div>
  <div class="boundary app"><h3>Your application owns</h3><ul><li>Transport and server ordering</li><li>Sessions, auth, and retries</li><li>History and storage</li><li>Presence, cursors, and editor rendering</li></ul></div>
</div>

Next, read [Document state](/docs/document/state) or [Values](/docs/core/values).
