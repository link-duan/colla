# Golden fixtures

Versioned, language-neutral fixtures that pin *given input → fixed expected
output / canonical bytes / error code*. They are reviewed regression evidence for
the reference implementation and its JavaScript facade — **not** a normative
specification and not proof that two independent implementations agree. When a
fixture disagrees with the [binary format](../docs/binary-format.md) or
[data model](../docs/data-model.md), those specifications win and the fixture is
treated as a defect.

The single fixture set is consumed from two sides:

- `crates/colla/tests/golden.rs` checks the Rust reference implementation
  (`cargo test --workspace`, or `cargo test -p colla --test golden`).
- `packages/core/tests/golden.test.mjs` checks the built `colla-ot` Node
  artifact and its JS↔Wasm boundary (`pnpm test:js`).

Sharing one data source is the point: the same JSON drives both sides, so any
divergence in canonical bytes or results surfaces immediately. The design
rationale lives in [`docs/internal/golden-tests.md`](../docs/internal/golden-tests.md);
the neutral representation and unified error codes are fixed by
[ADR 0006](../docs/adr/0006-specifications-testing-and-release-validation.md) and
[ADR 0005](../docs/adr/0005-errors-and-resource-lifecycle.md).

## Layout

```
golden/
  value-codec/*.json    # Value ↔ canonical bytes round-trip
  decode-error/*.json   # inputs the strict decoder rejects
  apply/*.json          # apply result or error
  compose/*.json        # compose result or error
  invert/*.json         # invert result and round-trip
  transform/*.json      # paired transform result and convergence
```

A fixture's `id` equals its path relative to `golden/` with the extension
removed (e.g. `value-codec/map-nested`); it is unique and stable.

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
[`docs/internal/golden-tests.md` §3](../docs/internal/golden-tests.md).

## Fixtures

A fixture's `id` and `kind` are derived from its path, not stored in the file:
the path under `golden/` (without the extension) is the `id`, and its first
segment is the `kind` (e.g. `apply/map-modify` → id `apply/map-modify`, kind
`apply`). A file holds only the kind-specific fields and an optional `note`:

```json
{ "note": "optional" }
```

Fixture kinds and their assertions are defined in
[`docs/internal/golden-tests.md` §4](../docs/internal/golden-tests.md). The
`value-codec` kind is:

```json
{ "value": <value>, "canonicalBytes": "<hex>" }
```

It asserts `encode(value) == canonicalBytes`, that `decode(canonicalBytes)` is
structurally equal to `value`, and that re-encoding the decoded value reproduces
`canonicalBytes` (canonical uniqueness).

## Error codes

Error fixtures assert a single stable `code`. The codes are owned by the `colla`
core crate's `ErrorCode`; fixtures assert only the subset that core operations can
produce. See [`docs/internal/golden-tests.md` §5](../docs/internal/golden-tests.md)
and [ADR 0005](../docs/adr/0005-errors-and-resource-lifecycle.md).

## How the fixtures are consumed

Both sides walk the same `golden/` tree, dispatch by `kind`, and call only the
public API. Each side **hard-fails** on an unknown `kind` or `code` so no case is
ever silently skipped. To run them directly:

```
cargo test -p colla --test golden                    # Rust reference
pnpm --filter colla-ot build                          # build the Wasm package first
node --test packages/core/tests/golden.test.mjs       # JavaScript facade
```

## Changing fixtures

Within the current pre-1.0 line only additive fixtures are allowed. Changing an
existing fixture's expected output must be a genuine behavior change, updated on
both sides in one commit and recorded in `CHANGELOG.md` when it affects canonical
bytes or semantics. The fixtures are not directory-versioned; if a wire-breaking
revision ever needs old and new vectors side by side, reintroduce a version layer
at that point. See
[`docs/internal/golden-tests.md` §7](../docs/internal/golden-tests.md).
