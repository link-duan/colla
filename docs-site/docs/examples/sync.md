# Two-client synchronization

Alice and Bob edit the same counter before either receives confirmation. A small in-memory exchange sends each Submission to the Authority and delivers its Commit to both clients.

## Example

<<< ../../examples/sync.ts

## Expected behavior

Both clients finish at `3n`: Alice contributes one and Bob contributes two. Receiving their own Commits confirms the edits already visible locally.

## Use it in your application

The exchange function shows message ordering without adding sockets or storage. In production, serialize server acceptance and persist the returned Authority before adopting it and broadcasting. Encode and decode messages at transport boundaries. See [Synchronization](/docs/sync/) for the full integration contract and [Retries](/docs/sync/retries) for delivery failures.
