# Production testing

Test the Core algebra and the application controller as separate layers.
Use Rust unit tests and JavaScript tests to cover shared Value and Change semantics.
Run golden fixtures for canonical bodies, projections, and stable error categories.
Round-trip every supported Value, Change, Snapshot, and Update fixture.
Assert that decoders reject wrong magic and unsupported versions.
Assert that truncated and trailing-byte payloads fail deterministically.
Exercise maximum unsigned revisions and checked integer overflow.
Verify that malformed remote input leaves Document state unchanged.

## Document scenarios

Test local edits with no pending queue.
Test one local edit followed by a server acknowledgement.
Test multiple local edits and out-of-order acknowledgements.
Test an ordered remote edit while local work is pending.
Test a skipped remote revision and request resynchronization.
Test listener failure isolation and error event delivery.
Test Snapshot restore with visible content and revision only.
Test idempotent disposal and operations after disposal.

## Browser and packaging

Test both `colla-ot` and `colla-ot/core` imports.
Test Node, Vite, and Rollup entry points used by consumers.
Verify synchronous Wasm initialization in each runtime.
Run a browser smoke test with the actual CDN or packaged artifact.
Check that the delivered Wasm bytes match the package build provenance.
Keep repository-wide pre-existing diagnostics separate from changed-file failures.

## Next

Use [errors and limits](./errors-limits) to turn failure cases into assertions.
Then review [persistence](./persistence) and [sync protocol](./sync-protocol).
