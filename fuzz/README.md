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

- `decode_value` — decode arbitrary bytes as a version 2 `Value` or
  `SyncSnapshot`. Accepted objects must preserve exact canonical input bytes
  through encode → decode, including all owning IDs and Ref targets.
- `decode_change` — the same invariants for `Change`, `Submission`,
  `ServerMessage`, `SessionCheckpoint`, `HistoryCheckpoint` and
  `AuthorityCheckpoint`. Old versions, wrong envelope types, malformed fields
  and trailing bytes must be rejected without a panic.
- `ot_algebra` — turn bytes into two valid multi-operation changes over the
  same identity-bearing List, mixing native Move, insertion, deletion, Set and
  Int Add. Assert codec round-trips, identity-preserving invert,
  `apply(apply(base, a), invert(base, a)) == base`, composition equivalence and
  TP1 for both priorities. This generator produces required-to-merge cases:
  transform or apply errors fail the target rather than skipping the assertion.

Rust integration/property tests additionally cover cross-parent and Map moves,
structural conflicts, Unicode Text/RichText, Ref, History and three-client sync.
These fuzz targets complement those tests; a bounded smoke run is not exhaustive.

Seed decoder corpora with the matching envelope bytes from `golden/v2.json` to
exercise valid protocol and checkpoint branches as well as malformed input.

## Running

Requires a nightly toolchain and [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz):

```sh
cargo install cargo-fuzz
python3 fuzz/seed.py
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
