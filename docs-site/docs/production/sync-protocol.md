# Synchronization protocol design

Colla requires remote Updates to arrive in confirmed-revision order.
The server or gateway should maintain one monotonic revision per document.
Accepted Changes must enter that same ordered sequence.

## Message flow

```text
client: edit -> Document.transact -> Update bytes
server: authenticate -> deduplicate -> order -> persist -> broadcast
client: receive bytes -> check revision -> applyRemote -> render editSteps
client: server acceptance -> ack(updateId) (cumulative)
```

The application protocol supplies document identity, authentication, authorization,
request identity, retry behavior, and response semantics.
The server supplies durable ordering and accepted-operation history.
The client reports its confirmed revision with each session request.
The client sends canonical Update bytes (`update.bytes`) through an application-owned queue.
The client applies only the next expected remote revision.

| Concern | Owner | Required behavior |
| --- | --- | --- |
| Content transition | Colla | Validate and apply the Change |
| Local visible state | Document | Rebase pending Changes |
| Request identity | Application | Deduplicate retries |
| Revision order | Server | Assign durable sequence |
| Transport | Application | Deliver and reconnect |
| Rendering | Editor adapter | Apply edit-step projection |

An Update's `updateId` may correlate a client acknowledgement.
It is local to one Document instance and is not a server deduplication key.
On a revision gap, replay missing history or request a new Snapshot.
Do not force `applyRemote()` against an unrelated revision.

## Next

Read [persistence](./persistence) for checkpoints.
Read [errors and limits](./errors-limits) for boundary validation.
Read [testing](./testing) for delivery scenarios.
