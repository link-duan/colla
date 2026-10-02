import { Authority, History, SessionCheckpoint, SyncSession } from 'colla-ot'

let authority = Authority.create({ documentId: 'history', value: { count: 0n } })
let alice = SyncSession.create({ clientId: 'alice', snapshot: authority.snapshot() })
const bob = SyncSession.create({ clientId: 'bob', snapshot: authority.snapshot() })
let history = History.attach(alice.document)
alice.document.edit(tx => tx.increment(['count'], 1n), { group: 'gesture-1' })
alice.document.edit(tx => tx.increment(['count'], 2n), { group: 'gesture-1' })
function flush(client: SyncSession): void {
  while (client.outbound()) {
    const result = authority.accept(client.outbound()!)
    if (result.message.type !== 'commit') throw result.message.reason
    // Persist result.authority.checkpoint() before adopting in production.
    authority = result.authority
    alice.receive(result.message)
    bob.receive(result.message)
  }
}
flush(alice)
bob.document.edit(tx => tx.increment(['count'], 10n))
flush(bob)
const saved = alice.checkpoint().encode() // includes attached History
alice.close()
alice = SyncSession.restore(SessionCheckpoint.decode(saved))
history = History.attach(alice.document) // obtain restored History
history.undo() // removes grouped +3, preserving Bob's +10
flush(alice)
if (alice.document.get(['count'])?.toJS() !== 10n) throw new Error('Undo removed remote work')
history.redo()
flush(alice)
if (bob.document.get(['count'])?.toJS() !== 13n) throw new Error('Redo was not synchronized')
history.close()
alice.close()
bob.close()
console.log('Grouped collaborative undo/redo and History restoration passed')
