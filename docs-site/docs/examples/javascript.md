# JavaScript examples

Start with [Document synchronization](./javascript-document) for optimistic local edits, ordered remote updates, and FIFO acknowledgements. Use [Core Value and Change](./javascript-core) when an application needs immutable values and OT algebra without Document state.

Both examples use ESM imports, `colla-ot` for the high-level API, and `colla-ot/core` for `ValueHandle`, `text`, and `Change` construction. Build the package before running examples with Node.js 22+.

## Runtime assumptions

The package is ESM-only.
Node imports use the Node entry point.
Browser bundlers use the browser entry point.
Both entries initialize the same Wasm binary.
No public Wasm initialization function is required.
The examples use synchronous API calls after import.
Values are recursively immutable.
Changes are recursively canonical.
Plain strings are atomic values.
`text()` enables scalar sequence editing.
Map keys are explicit.
List operations retain stream order.
RichText embeds are atomic sequence units.
Coordinates are converted at the editor boundary.
Errors expose stable `CollaError` codes.
Handles may be disposed explicitly.

## Suggested progression

Run the Core example first.
Inspect the resulting Value.
Build a Document around the same Change.
Add a transport queue outside Document.
Add ordered remote delivery.
Add Snapshot persistence.
Add retry and reconnect policy.
Add application authentication.
Add production tests.

## Next

Read [Document synchronization](./javascript-document).
Read [Core operations](./javascript-core).
Read [editor integration](./editor-integration).
