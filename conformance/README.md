# Conformance corpus

A language-neutral, machine-executable specification: versioned fixtures that
describe *given input → expected output / canonical bytes / error code*, run by
both the Rust and JavaScript implementations to prove they share one data model
and OT semantics.

The design rationale lives in [`docs/internal/conformance.md`](../docs/internal/conformance.md);
the format and runner contract are fixed by
[ADR 0014](../docs/adr/0014-conformance-corpus-format-and-runner-contract.md) and
the error classification by
[ADR 0015](../docs/adr/0015-error-code-classification.md). This file is the
normative description of the on-disk format. When it disagrees with the
[binary format](../docs/binary-format.md) or [data model](../docs/data-model.md),
those specifications win.

## Layout

```
conformance/
  corpus/
    v1/
      value-codec/*.json    # Value ↔ canonical bytes round-trip
      decode-error/*.json   # inputs the strict decoder rejects
      apply/*.json          # apply result or error
      compose/*.json        # compose result or error
      invert/*.json         # invert result and round-trip
      transform/*.json      # paired transform result and convergence
  runners/
    rust/                   # reference runner (`cargo test -p colla-conformance`)
    js/                     # JavaScript runner (added in a later phase)
```

A fixture's `id` equals its path relative to `corpus/vN/` with the extension
removed (e.g. `value-codec/map-nested`); it is unique and stable across the
corpus.

## Neutral encoding

Fixtures describe Values and Changes with tagged JSON so they never depend on
JSON's native types (JSON cannot tell Int from Float, or String from Text).

Each **Value** is an object with exactly one key — the type tag:

| Tag | Form | Notes |
| --- | --- | --- |
| `null` | `{"null": null}` | |
| `bool` | `{"bool": true}` | |
| `int` | `{"int": "-42"}` | decimal string, full `i64` precision |
| `float` | `{"float": 1.5}` | finite JSON number; `-0.0` normalizes to `0.0` |
| `string` | `{"string": "abc"}` | atomic string |
| `text` | `{"text": "abc"}` | character-OT text |
| `richtext` | `{"richtext": [<span>]}` | span sequence |
| `list` | `{"list": [<value>]}` | |
| `map` | `{"map": {"k": <value>}}` | JSON object; key order carries no meaning |

Maps use JSON objects for readability. Key-ascending order is the encoding-layer
canonical form, asserted by `canonicalBytes` rather than by JSON key order;
constructors sort keys. Non-canonical inputs (reordered or duplicate keys) are
expressed as raw `decode-error` byte inputs instead. Fixtures must not contain
duplicate JSON keys.

Canonical binary bodies are lowercase hex strings with no separators
(e.g. `"0503616263"`).

The full Value and Change grammar (RichText spans, Change ops, `mapEntryChange`,
`attrPatch`, …) is specified in
[`docs/internal/conformance.md` §3](../docs/internal/conformance.md).

## Fixtures

Every fixture shares an envelope:

```json
{ "id": "...", "kind": "...", "corpusVersion": 1, "note": "optional" }
```

`corpusVersion` must match the enclosing `corpus/vN` directory. Fixture kinds and
their assertions are defined in
[`docs/internal/conformance.md` §4](../docs/internal/conformance.md). The
`value-codec` kind, exercised by the phase-1 runner, is:

```json
{ "kind": "value-codec", "value": <value>, "canonicalBytes": "<hex>" }
```

It asserts `encode(value) == canonicalBytes`, that `decode(canonicalBytes)` is
structurally equal to `value`, and that re-encoding the decoded value reproduces
`canonicalBytes` (canonical uniqueness).

## Error codes

Error fixtures assert a single stable `code` shared by both implementations. The
codes are owned by the `colla` core crate's `ErrorCode`; the corpus asserts only
the subset that core operations can produce. See
[`docs/internal/conformance.md` §5](../docs/internal/conformance.md) and
[ADR 0015](../docs/adr/0015-error-code-classification.md).

## Runner contract

The corpus is the only source of truth; runners stay thin. Each runner walks the
same `corpus/vN/`, dispatches by `kind`, and calls only its implementation's
public API. A runner **must hard-fail** on an unknown `kind` or `code` so no case
is ever silently skipped. The Rust runner is the reference implementation; when
the two runners disagree on a fixture, that is adjudicated as a specification
defect per [ADR 0010](../docs/adr/0010-normative-specification-and-conformance.md).

Both runners are part of the regular test suites: the Rust runner is a workspace
member, so `cargo test --workspace` runs it, and the JavaScript runner is invoked
by `colla-ot`'s `pnpm test`. To run them directly:

```
cargo test -p colla-conformance          # Rust reference runner
pnpm --filter colla-ot build             # build the WebAssembly package first
node --test conformance/runners/js/corpus.test.mjs   # JavaScript runner
```

## Versioning

Fixtures are versioned by the `corpus/vN/` directory, and within one version only
additive fixtures are allowed. Changing an existing fixture's expected output must
be a genuine specification change, updated in both runners in one commit and
recorded in `CHANGELOG.md` when it affects canonical bytes or semantics. A
wire-breaking revision adds `corpus/v2/` alongside `v1/`. See
[`docs/internal/conformance.md` §7](../docs/internal/conformance.md).
