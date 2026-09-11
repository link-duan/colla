import { Authority, CollaError, Document, SyncSession } from 'colla-ot'

function check(condition: boolean, message: string) {
  if (!condition) throw new Error(message)
}

// Compaction can reject an old request. Receiving the rejection must retain the
// optimistic work, stop sending and leave other writers alone.
let authority = Authority.create({ documentId: 'recovery', value: { count: 0n } })
const baseline = authority.snapshot()
const stale = SyncSession.create({ clientId: 'stale', snapshot: baseline })
const live = SyncSession.create({ clientId: 'live', snapshot: baseline })
stale.document.edit(tx => tx.increment(['count'], 1n))
live.document.edit(tx => tx.increment(['count'], 10n))
const accepted = authority.accept(live.outbound()!)
authority = accepted.authority
live.receive(accepted.message)
authority = authority.compact(authority.revision)
const rejected = authority.accept(stale.outbound()!).message
check(rejected.type === 'rejection' && rejected.reason?.code === 'history_expired', 'Expected history rejection')
const before = stale.document.snapshot()
check(stale.receive(rejected) === null, 'Rejection should not be a content edit')
check(stale.state.status === 'recovery-required', 'Rejected session stayed active')
check(stale.state.recoveryReason?.code === 'history_expired', 'Recovery reason was lost')
check(!stale.state.hasOutbound && stale.outbound() === null, 'Rejected session still sends')
check(stale.document.snapshot().equals(before), 'Rejection discarded content or IDs')
live.receive(rejected)
check(live.state.status === 'active', 'Unrelated rejection changed another writer')
stale.document.edit(tx => tx.increment(['count'], 2n))
check(stale.document.get(['count'])?.toJS() === 3n, 'Recovery blocked local work')
stale.close()
live.close()

// Copying the root into a descendant creates a finite copy of the prior tree.
const doc = Document.create({ title: 'Draft', archive: [], items: ['A', 'B', 'C'] })
const frozen = doc.snapshot()
doc.edit(tx => tx.copy([], { parent: ['archive'], index: 0 }))
const archived = doc.get(['archive', 0])!
check(archived.id !== frozen.id && archived.contentEquals(frozen), 'Root Copy changed source content')
check(doc.idAt(['archive', 0, 'items', 0]) !== doc.idAt(['items', 0]), 'Root Copy reused descendants')
const moved = doc.idAt(['items', 0])
doc.edit(tx => tx.move(moved, { parent: ['items'], index: 2 }))
check(JSON.stringify(doc.get(['items'])?.toJS()) === '["B","C","A"]', 'Move index is not post-removal')
check(doc.idAt(['items', 2]) === moved, 'Move lost identity')

const originalTitle = doc.idAt(['title'])
doc.edit(tx => {
  tx.set(['title'], 'Ready')
  tx.set(['approved'], true)
  tx.list(['items']).insert(3, ['D'])
})
check(doc.idAt(['title']) === originalTitle && doc.has(['approved']), 'Map Set semantics changed')
const oldItem = doc.idAt(['items', 0])
doc.edit(tx => tx.list(['items']).replace(0, 1, ['B']))
check(doc.idAt(['items', 0]) !== oldItem, 'List replacement retained removed identity')
for (const edit of [
  () => doc.edit(tx => { tx.set(['title'], 'invalid'); tx.set(['items', 4], 'E') }),
  () => doc.edit(tx => tx.set(['missing', 'child'], true)),
  () => doc.edit(tx => tx.list(['items']).delete(3, 2)),
]) {
  const beforeFailure = doc.snapshot()
  const version = doc.version
  let failed = false
  try { edit() } catch (error) {
    if (!(error instanceof CollaError)) throw error
    failed = true
  }
  check(failed, 'Invalid Map/List edit unexpectedly succeeded')
  check(doc.snapshot().equals(beforeFailure) && doc.version === version, 'Failed edit committed partial state')
}
doc.close()
console.log('Rejection recovery, root Copy, Move coordinates and Map/List boundaries passed')
