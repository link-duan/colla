# Colla Documentation

This index is the entry point for Colla's user guides, normative
specifications, architecture decisions, and project operations.

## User guides

- [Project overview and quick starts](../README.md)
- [`colla` Rust guide](../crates/colla/README.md)
- [`colla-ot` JavaScript guide](../packages/core/README.md) — application-oriented
  Document workflows and the low-level Core API.
- [`colla` Rust API reference](https://docs.rs/colla)

User guides explain how to consume the public APIs. They are not the normative
definition of the data model or wire format.

## Normative specifications

- [Core data model](data-model.md) — valid Value and Change structures,
  canonical form, and public semantic boundaries.
- [Document model](document-model.md) — high-level Snapshot, Update, Document,
  and local envelope codec.
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
- [Golden fixtures design](internal/golden-tests.md) — fixture format,
  error taxonomy, and how both sides consume the shared golden fixtures.
- [Coordinated release runbook](internal/releasing.md) — publishing the Rust
  crate and npm package from one version and tag.
- [Changelog](../CHANGELOG.md) — released history and current public changes.

## Architecture decisions

Accepted decisions:

- [ADR-0001: 公共 API 与跨运行时边界](adr/0001-public-api-and-runtime-boundaries.md)
- [ADR-0002: Core Value、Change 与 JavaScript 边界](adr/0002-core-value-change-and-js-boundary.md)
- [ADR-0003: Canonical codec 单一实现与 wire ownership](adr/0003-single-source-codec-and-wire-ownership.md)
- [ADR-0004: Document 与 Snapshot/Update 模型](adr/0004-document-snapshot-update-model.md)
- [ADR-0005: 稳定错误与 Wasm 资源生命周期](adr/0005-errors-and-resource-lifecycle.md)
- [ADR-0006: 规范、测试与发布验收边界](adr/0006-specifications-testing-and-release-validation.md)
