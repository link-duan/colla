# Security, limits, and error handling

Treat Snapshot, Update, and Core bytes as untrusted input. Decoders reject invalid magic or versions, truncated or malformed payloads, and trailing bytes. `InputOptions.limits` applies to structured JavaScript input passed to `ValueHandle.fromJS`, `Change.fromJS`, and `Change.build`; byte payload size and decode-resource limits must be enforced by the application at its input boundary.

Handle errors by matching `CollaError.code`, `operation`, optional `path`, and frozen `details`; do not depend on message text. Typical classifications include `invalid_encoding`, `type_mismatch`, `out_of_bounds`, `incompatible_change`, `limit_exceeded`, and `invalid_state`.

External services must also implement authentication, authorization, tenant isolation, request-size/rate limits, and auditing at the application layer. If remote apply fails, keep the current Document unchanged and enter resynchronization; do not mask the error by writing placeholder data.

## Boundary controls

Authenticate before reading a document.
Authorize every document operation.
Isolate tenant storage and queue records.
Limit request and response sizes.
Limit structured-input depth and collection lengths through `InputOptions`.
Limit request and byte-payload size in the application before decode.
Rate-limit repeated malformed requests.
Avoid logging content or credentials.
Redact identifiers in error telemetry where required.
Preserve the last known-good Document state.
Request a Snapshot after an unrecoverable revision gap.
Never substitute placeholder content for a rejected Change.
Treat error details as diagnostic data.
Match stable codes instead of message text.
Audit protocol version changes.
Verify delivered Wasm artifacts.
Review dependencies before release.

## Next

Read [errors and limits](./errors-limits).
Read [sync protocol](./sync-protocol).
Read [production testing](./testing).
Keep secrets out of Colla values.
Do not log raw document payloads by default.
Use tenant-scoped storage keys.
Rotate application credentials independently.
Review limits after workload measurements.
Test denial-of-service boundaries.
Test recovery after rejected input.
