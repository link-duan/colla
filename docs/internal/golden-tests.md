# Golden fixture workflow

The [shared fixture guide](../../golden/README.md) describes generation and review.
The version-2 corpus covers identities, refs, moves, text, formatting, inversion,
composition and every protocol/checkpoint type. Rust and JavaScript assert exact
bytes. Property and fuzz tests supplement these fixed examples; neither fixed
fixtures nor random tests alone establish the full concurrency contract.
