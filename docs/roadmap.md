# Colla Core Roadmap

This roadmap describes future work only. Released behavior belongs in the
[changelog](../CHANGELOG.md), while normative behavior belongs in the
[specifications](README.md#normative-specifications).

Colla remains focused on foundational OT primitives: Core Value, Change, OT
algebra, canonical codecs, and the official Rust and JavaScript APIs. Document,
Session, history, synchronization protocols, transport, presence, cursors, and
editor adapters remain outside this repository's milestones.

## Current: Colla Core 0.2 Hardening

Goal: use the pre-1.0 adjustment window to simplify public APIs and strengthen
the correctness, resource, compatibility, and performance evidence behind the
core model.

Planned work:

- refine Rust and JavaScript APIs based on real consumption, documenting all
  breaking changes and migrations;
- expand property, malformed-input, fuzz, and cross-language conformance tests;
- run Chromium, Firefox, and WebKit coverage for main-thread and Worker package
  entry points where the host supports them;
- verify long-running memory behavior, explicit disposal, clone independence,
  and error-path resource release;
- establish reviewed artifact-size and performance baselines before enforcing
  regression budgets.

Completion requires repeatable test evidence for the known correctness,
resource-lifecycle, browser, packaging, and performance risks. Baselines may be
updated only through explicit review.

## Next: Colla Core 1.0 Stability

Goal: freeze a durable public contract after the 0.2 hardening evidence and
real-world API feedback are sufficient.

Planned work:

- complete the human-readable data model, OT, and binary format specifications;
- publish a versioned conformance corpus and runner contract shared by Rust and
  JavaScript;
- freeze public API, error, semantic, and wire-compatibility commitments;
- confirm supported Rust, Node.js, bundler, and browser baselines at release
  time;
- publish migration guidance and a long-term maintenance policy.

Colla 1.0 is an evidence-based milestone, not a scheduled feature bundle. Extra
0.x iterations may be added when hardening reveals unresolved foundational
issues. New Value kinds or OT operations require separate consumer-driven
proposals and do not enter the roadmap automatically.
