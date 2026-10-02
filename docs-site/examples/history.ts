import { Authority, History, SyncSession } from 'colla-ot'

let authority = Authority.create({ documentId: 'counter', value: { count: 0n } })
const snapshot = authority.snapshot()
const alice = SyncSession.create({ clientId: 'alice', snapshot })
const bob = SyncSession.create({ clientId: 'bob', snapshot })
const history = History.attach(alice.document)

function synchronize(client: SyncSession) {
  const submission = client.outbound()
  if (!submission) return

  const { authority: next, message } = authority.accept(submission)
  if (message.type === 'rejection') {
    client.receive(message)
    console.log('Synchronization stopped:', client.state.status, client.state.recoveryReason?.code)
    return
  }

  // In production, persist next before adopting it and broadcasting the message.
  authority = next
  alice.receive(message)
  bob.receive(message)
}

alice.document.edit(tx => tx.increment(['count'], 1n))
synchronize(alice)
bob.document.edit(tx => tx.increment(['count'], 10n))
synchronize(bob)
console.log('Before undo — both contributions:', bob.document.get(['count'])?.toJS()) // 11n

history.undo() // Remove Alice's +1, keeping Bob's +10.
synchronize(alice)
console.log('After undo — Bob’s contribution remains:', bob.document.get(['count'])?.toJS()) // 10n

history.redo()
synchronize(alice)
console.log('After redo — Alice’s contribution restored:', bob.document.get(['count'])?.toJS()) // 11n

history.close()
alice.close()
bob.close()
