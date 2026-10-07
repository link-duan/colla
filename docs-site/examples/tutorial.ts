// #region model
import { Authority, CollaError, Document, History, SyncSession, text } from 'colla-ot'

const doc = Document.create({
  title: text('Groceries'),
  items: [{ name: text('Milk'), done: false }],
})
console.log('First item:', doc.get(['items', 0, 'name'])?.toJS())
// #endregion model

// #region edit
doc.edit(tx => {
  tx.list(['items']).insert(1, [{ name: text('Eggs'), done: false }])
  tx.set(['items', 0, 'done'], true)
  tx.text(['title']).insert(9, ' for Sunday')
})
console.log('Title:', doc.get(['title'])?.toJS())
console.log('Milk done:', doc.get(['items', 0, 'done'])?.toJS())
// #endregion edit

// #region rollback
try {
  doc.edit(tx => {
    tx.set(['items', 1, 'done'], true)
    tx.delete(['items', 5]) // There is no sixth item.
  })
} catch (error) {
  console.log('Edit failed:', (error as CollaError).code)
}
console.log('Eggs done:', doc.get(['items', 1, 'done'])?.toJS()) // Unchanged.
// #endregion rollback

// #region subscribe
const unsubscribe = doc.subscribe(event => {
  console.log('Committed:', event.origin, 'version', event.version)
})
// #endregion subscribe

// #region undo
const history = History.attach(doc)
doc.edit(tx => tx.delete(['items', 0]))
console.log('First item after delete:', doc.get(['items', 0, 'name'])?.toJS())
history.undo()
console.log('First item after undo:', doc.get(['items', 0, 'name'])?.toJS())
// #endregion undo

// #region sync
let authority = Authority.create({ documentId: 'groceries', value: doc.snapshot() })
const alice = SyncSession.create({ clientId: 'alice', snapshot: authority.snapshot() })
const bob = SyncSession.create({ clientId: 'bob', snapshot: authority.snapshot() })

// Both users edit the same snapshot at the same time.
alice.document.edit(tx => tx.text(['items', 0, 'name']).insert(4, ' (oat)'))
bob.document.edit(tx => tx.list(['items']).insert(0, [{ name: text('Bread'), done: false }]))

// Stand-in for your network: send pending work to the Authority, broadcast the Commit.
function deliver(client: SyncSession) {
  const submission = client.outbound()
  if (!submission) return
  const { authority: next, message } = authority.accept(submission)
  if (message.type === 'rejection') throw new Error('submission rejected')
  authority = next
  alice.receive(message)
  bob.receive(message)
}
deliver(alice)
deliver(bob)
console.log('Alice, first item:', alice.document.get(['items', 0, 'name'])?.toJS())
console.log('Alice, second item:', alice.document.get(['items', 1, 'name'])?.toJS())
console.log('Bob, second item:', bob.document.get(['items', 1, 'name'])?.toJS())
// #endregion sync

// #region close
unsubscribe()
history.close()
doc.close()
alice.close()
bob.close()
// #endregion close
