# Colla Documentation

This index is the entry point for Colla's user guides, normative
specifications, architecture decisions, and project operations.

## User guides

- [Project overview and quick starts](../README.md)
- [`colla` Rust guide](../crates/colla/README.md)
- [`colla-ot` JavaScript guide](../packages/core/README.md)
- [`colla` Rust API reference](https://docs.rs/colla)

User guides explain how to consume the public APIs. They are not the normative
definition of the data model or wire format.

## Normative specifications

- [Core data model](data-model.md) — valid Value and Change structures,
  canonical form, and public semantic boundaries.
- [OT properties](ot-properties.md) — Apply, Compose, Invert, TP1, conflict
  behavior, and the explicit TP2 limitation.
- [Canonical binary body format](binary-format.md) — the unique Value and
  Change byte encoding and strict decoder requirements.

These documents define behavior. Architecture decisions explain why the
project chose a particular design; implementation details and examples do not
override the specifications.

## Domain language

- [Ubiquitous language](../CONTEXT.md) — canonical project terms and terms to
  avoid.

## Project operations

- [Roadmap](internal/roadmap.md) — future 0.2 hardening and 1.0 stability work.
- [Conformance corpus plan](internal/conformance.md) — planned fixture format,
  error taxonomy, and runner contract for the shared Rust/JavaScript corpus.
- [Coordinated release runbook](internal/releasing.md) — publishing the Rust
  crate and npm package from one version and tag.
- [Changelog](../CHANGELOG.md) — released history and current public changes.

## Architecture decisions

Accepted decisions:

- [ADR-0001: Rust and JavaScript use a functional public API](adr/0001-function-oriented-cross-language-api.md)
- [ADR-0002: Rust, Wasm, and TypeScript keep one-way boundaries](adr/0002-rust-wasm-typescript-boundaries.md)
- [ADR-0003: The JavaScript Value boundary is strict and canonical](adr/0003-canonical-javascript-value-boundary.md)
- [ADR-0006: JavaScript uses stable errors and explicit resource ownership](adr/0006-errors-and-resource-lifecycle.md)
- [ADR-0007: One Wasm artifact provides synchronous cross-runtime entry points](adr/0007-synchronous-cross-runtime-wasm-package.md)
- [ADR-0008: Release artifacts are the workspace acceptance boundary](adr/0008-workspace-testing-and-coordinated-release.md)
- [ADR-0009: Pre-1.0 compatibility is promised within patch lines](adr/0009-pre-1.0-compatibility-policy.md)
- [ADR-0010: Written specifications and conformance fixtures define the 1.0 contract](adr/0010-normative-specification-and-conformance.md)
- [ADR-0011: RichText logical spans are separate from indexed storage](adr/0011-richtext-logical-spans-and-indexed-storage.md)
- [ADR-0012: JavaScript Change uses Snapshot-independent typed construction](adr/0012-snapshot-independent-change-construction.md)
- [ADR-0013: Change construction and Snapshot projection use distinct coordinates](adr/0013-change-and-projection-coordinates.md)
- [ADR-0014: Conformance corpus uses a neutral tagged representation and unified error codes](adr/0014-conformance-corpus-format-and-runner-contract.md)
- [ADR-0015: The core crate owns the ErrorCode classification; the TS type is maintained separately](adr/0015-error-code-classification.md)

Superseded decisions remain available as historical context:

- [ADR-0004: ChangeBuilder was relative to a Snapshot](adr/0004-snapshot-relative-fluent-builders.md) — superseded by ADR-0012.
- [ADR-0005: JavaScript text APIs used UTF-16 coordinates](adr/0005-text-richtext-coordinates-and-embeds.md) — superseded by ADR-0013.
