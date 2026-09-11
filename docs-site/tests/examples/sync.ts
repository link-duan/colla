import { Authority, AuthorityCheckpoint, CollaError, ServerMessage, SessionCheckpoint,
  Submission, SyncSession } from 'colla-ot'

let authority = Authority.create({ documentId: 'counter', value: { count: 0n } })
const baseline = authority.snapshot()
let alice = SyncSession.create({ clientId: 'alice', snapshot: baseline })
const bob = SyncSession.create({ clientId: 'bob', snapshot: baseline })
alice.document.edit(tx => tx.increment(['count'], 1n))
bob.document.edit(tx => tx.increment(['count'], 2n))
const original = alice.outbound()!.encode()
alice.document.edit(tx => tx.increment(['count'], 3n)) // buffered while first request is in flight

function exchange(client: SyncSession): ServerMessage {
  const submission = client.outbound()
  if (!submission) throw new Error('Expected outbound work')
  const result = authority.accept(Submission.decode(submission.encode()))
  if (result.message.type !== 'commit') throw result.message.reason
  const durableBytes = result.authority.checkpoint().encode()
  // Demo storage boundary. In production await a durable database write here.
  authority = Authority.restore(AuthorityCheckpoint.decode(durableBytes))
  const message = ServerMessage.decode(result.message.encode())
  alice.receive(message)
  bob.receive(message)
  return message
}
exchange(bob)
if (String(alice.outbound()!.encode()) !== String(original)) throw new Error('Retry payload changed')
const checkpoint = alice.checkpoint().encode()
alice.close() // stop the original writer before restoring the same client ID
alice = SyncSession.restore(SessionCheckpoint.decode(checkpoint))
if (String(alice.outbound()!.encode()) !== String(original)) throw new Error('Restore lost retry payload')
const first = exchange(alice)
const duplicate = authority.accept(Submission.decode(original))
if (duplicate.authority.revision !== authority.revision) throw new Error('Duplicate advanced revision')
if (String(duplicate.message.encode()) !== String(first.encode())) throw new Error('Duplicate changed receipt')
alice.receive(first) // duplicate delivery is safe
const last = exchange(alice) // buffered +3 is now a separate submission
for (const client of [alice, bob]) {
  if (!client.document.snapshot().equals(authority.snapshot().value)) throw new Error('Clients diverged')
  if (client.document.get(['count'])?.toJS() !== 6n) throw new Error('Lost an edit')
}
const late = SyncSession.create({ clientId: 'late', snapshot: baseline })
const beforeGap = late.document.snapshot()
try {
  late.receive(last)
  throw new Error('Expected missing revision')
} catch (error) {
  if (!(error instanceof CollaError) || error.code !== 'missing_revision') throw error
}
if (!late.document.snapshot().equals(beforeGap)) throw new Error('Gap changed content')
for (const message of authority.commitsSince(late.revision)) late.receive(message)
if (!late.document.snapshot().equals(authority.snapshot().value)) throw new Error('Catch-up failed')
for (const client of [alice, bob, late]) client.close()
console.log('Both clients converged to 6; retry, restart and gap recovery passed')
