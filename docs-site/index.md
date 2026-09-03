---
layout: home
pageClass: landing-page
title: Colla
titleTemplate: Operational Transformation Core
description: High-performance real-time collaboration for structured documents.
hero:
  text: High-performance real-time collaboration for structured documents.
  tagline: A lock-free Operational Transformation engine built in Rust. Microsecond conflict resolution, pure synchronous execution, and byte-for-byte determinism across peers.
  actions:
    - theme: brand
      text: Get started →
      link: /docs/getting-started
    - theme: alt
      text: API reference
      link: /reference/javascript
    - theme: alt
      text: GitHub
      link: https://github.com/link-duan/colla
---

<div class="home-body">

<div class="home-hero-meta">
  <div class="home-install-row">
    <div class="install-pill" onclick="navigator.clipboard.writeText('npm install colla-ot')" title="Click to copy npm install command">
      <svg class="install-brand-icon npm-icon" viewBox="0 0 256 256" width="14" height="14" aria-hidden="true"><rect width="256" height="256" rx="36" fill="#CB3837"/><path d="M48 48h160v160h-48V88h-32v120H48z" fill="#FFF"/></svg>
      <span class="install-cmd">npm install colla-ot</span>
      <span class="copy-hint" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>
      </span>
    </div>
    <div class="install-pill" onclick="navigator.clipboard.writeText('cargo add colla')" title="Click to copy cargo add command">
      <svg class="install-brand-icon cargo-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#E5532A" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m7.5 4.27 9 5.15"/><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/></svg>
      <span class="install-cmd">cargo add colla</span>
      <span class="copy-hint" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/></svg>
      </span>
    </div>
  </div>
  <div class="home-eco-bar">
    <span class="eco-tag">Packages</span>
    <a class="eco-link" href="https://www.npmjs.com/package/colla-ot" target="_blank" rel="noopener">npm ↗</a>
    <span class="eco-dot">·</span>
    <a class="eco-link" href="https://crates.io/crates/colla" target="_blank" rel="noopener">crates.io ↗</a>
    <span class="eco-dot">·</span>
    <a class="eco-link" href="https://github.com/link-duan/colla/blob/master/CHANGELOG.md" target="_blank" rel="noopener">Changelog</a>
  </div>
</div>

<div class="home-features">
  <div class="feature-col">
    <div class="feature-header">
      <span class="feature-idx">01</span>
      <h3>Unified model, native speed</h3>
    </div>
    <p>Identical behavior across Rust, Node.js, and browsers. Pure synchronous execution with microsecond transforms and zero async ceremony.</p>
  </div>
  <div class="feature-col">
    <div class="feature-header">
      <span class="feature-idx">02</span>
      <h3>Deterministic OT</h3>
    </div>
    <p>Pairwise algebraic transformation with strict convergence guarantees. Invertible operations deliver instantaneous local undo/redo stacks.</p>
  </div>
  <div class="feature-col">
    <div class="feature-header">
      <span class="feature-idx">03</span>
      <h3>Structured & RichText</h3>
    </div>
    <p>Native support for text, rich-text attribute spans, and immutable value trees. Eliminates format splitting anomalies across peers.</p>
  </div>
</div>

<div class="home-code-section">
  <div class="code-pane-header">
    <span class="pane-title">quickstart.ts</span>
    <span class="pane-lang">TypeScript</span>
  </div>

```ts
import { Document, Change, text } from 'colla-ot'

// 1. Initialize document and apply optimistic local change
const doc = Document.fromJS(text('Draft'))
const update = doc.applyLocal(Change.build(b => b.text(t => t.retain(5).insert(' v2'))))

// 2. Encode canonical binary update bytes for transport
const bytes = update.encode()

// 3. Acknowledge when server confirms ordering
doc.ack(update.updateId)
```

</div>

</div>
