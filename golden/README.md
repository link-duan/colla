# Shared v2 fixtures

`v2.json` is generated from deterministic element allocation by the Rust public
API. It locks all eight object envelopes and base-aware apply, compose, invert,
and transform examples (Move+edit, competing text and ancestor deletion).
`crates/colla/tests/golden.rs` rebuilds the examples and compares exact bytes;
`packages/core/tests/golden.test.mjs` decodes the same data and checks the same
outputs through the public JavaScript package.

Run `cargo test -p colla --test golden` and `pnpm test:js`. To intentionally
regenerate after reviewing a format change, use
`UPDATE_GOLDEN=1 cargo test -p colla --test golden`, then inspect the diff and
rerun both suites. Old-format fixtures were retired with the breaking upgrade.
