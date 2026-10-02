import { Authority, SyncSession } from 'colla-ot'

let authority = Authority.create({ documentId: 'counter', value: { count: 0n } })
const snapshot = authority.snapshot()
const alice = SyncSession.create({ clientId: 'alice', snapshot })
const bob = SyncSession.create({ clientId: 'bob', snapshot })

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
  console.log('Confirmed server revision:', authority.revision)
}

alice.document.edit(tx => tx.increment(['count'], 1n))
bob.document.edit(tx => tx.increment(['count'], 2n))

// Local edits are already visible before the server confirms them.
console.log('Before sync — Alice:', alice.document.get(['count'])?.toJS()) // 1n
console.log('Before sync — Bob:', bob.document.get(['count'])?.toJS()) // 2n

synchronize(alice)
synchronize(bob)

console.log('After sync — Alice:', alice.document.get(['count'])?.toJS()) // 3n
console.log('After sync — Bob:', bob.document.get(['count'])?.toJS()) // 3n

alice.close()
bob.close()
