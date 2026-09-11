# Retries and revision gaps

Networks can lose, duplicate or reorder messages. Colla provides stable request identity,
idempotent receipts and explicit revision gaps; the application supplies delivery policy.

## Retry the original bytes

Repeated outbound calls return the original in-flight Submission. Persist the complete
SessionCheckpoint so a restart retains those bytes and its next request sequence. New
local edits are buffered; incoming remote work updates a separate rebased working Change.
Do not regenerate a Submission from current visible content or allocate a new client ID
just because a reply was lost.

Authority deduplicates a previously accepted request and returns its receipt without
advancing revision. Delivering the same Commit again is safe for the session. Use bounded
retry delays and reconnection logic in the transport, not inside a Transaction callback.

## Fill a revision gap

Receiving a later Commit before its predecessors raises missing_revision with fromRevision
and throughRevision in error details. Visible content stays unchanged. Request the missing
interval from the server, receive those Commits in order, then retry the delayed message.
Authority.commitsSince can provide the retained suffix; filter to the requested interval
at your transport boundary if appropriate.

## When retry cannot help

If the required history has expired, the session cannot safely infer a new confirmed
baseline from its visible content. Follow [Recovery](./recovery), retaining local work.
Do not treat a Value snapshot as an acknowledgement or silently clear pending state.
Start with the [two-client example](/docs/examples/sync) for the basic exchange, then
add retry scheduling and missing-interval delivery to your transport.
