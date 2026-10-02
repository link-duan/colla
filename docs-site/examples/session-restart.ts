import { Authority, SessionCheckpoint, SyncSession } from 'colla-ot'

const authority = Authority.create({ documentId: 'counter', value: { count: 0n } })
const session = SyncSession.create({ clientId: 'alice', snapshot: authority.snapshot() })
session.document.edit(tx => tx.increment(['count'], 1n))
console.log('Before restart:', session.document.get(['count'])?.toJS()) // 1n
const bytes = session.checkpoint().encode() // Persist these bytes in application storage.
session.close() // Stop the old writer before restoring its identity.

const resumed = SyncSession.restore(SessionCheckpoint.decode(bytes))
console.log('After restart:', resumed.document.get(['count'])?.toJS()) // 1n
console.log('Still awaiting confirmation:', resumed.state.hasOutbound) // true
resumed.close()
