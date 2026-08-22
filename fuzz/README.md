# Colla fuzzing

Coverage-guided fuzz targets that back the roadmap's "fuzz coverage" hardening
item with repeatable evidence. They cover two layers:

- **Decoder boundary** — untrusted bytes must never panic, and every value the
  strict decoder accepts must be canonical.
- **OT algebra** — structured, coverage-guided exploration of the semantic core
  (apply, compose, invert, transform) with generated-but-valid inputs.

This is a standalone crate, detached from the main workspace, so
`cargo test --workspace` never builds the nightly-only libFuzzer targets.

## Targets

- `decode_value` — decode arbitrary bytes as a `Value`; any accepted value must
  survive a decode → encode → decode round-trip with byte-stable output.
- `decode_change` — the same invariants for `Change`.
- `ot_algebra` — interpret the input as an `arbitrary` stream that builds a
  valid base `Value` and compatible `Change`s, then assert the OT laws: codec
  round-trips, the invert law `apply(apply(v, a), invert(a, v)) == v`, the
  compose law `apply(v, compose(a, b)) == apply(apply(v, a), b)`, and TP1
  convergence for two concurrent changes. Deeper laws are gated on the relevant
  `apply` succeeding, so every failure is a genuine engine invariant violation.

The `ot_algebra` target reaches far more of the crate than the byte decoders
(roughly 2000+ edges versus a few hundred), because it spends its bytes on the
semantic operations rather than on getting past the parser.

## Running

Requires a nightly toolchain and [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz):

```sh
cargo install cargo-fuzz
cargo +nightly fuzz run decode_value
cargo +nightly fuzz run decode_change
cargo +nightly fuzz run ot_algebra
```

Bound a session for a smoke run:

```sh
cargo +nightly fuzz run ot_algebra -- -max_total_time=60
```

Reproduce a saved crash with its artifact path:

```sh
cargo +nightly fuzz run decode_value fuzz/artifacts/decode_value/crash-<hash>
```
