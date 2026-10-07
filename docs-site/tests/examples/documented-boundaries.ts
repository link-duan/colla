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
check(
  rejected.type === 'rejection' && rejected.reason?.code === 'history_expired',
  'Expected history rejection',
)
const before = stale.document.snapshot()
check(stale.receive(rejected) === null, 'Rejection should not be a content edit')
check(stale.state.status === 'recovery-required', 'Rejected session stayed active')
check(stale.state.recoveryReason?.code === 'history_expired', 'Recovery reason was lost')
check(!stale.state.hasOutbound && stale.outbound() === null, 'Rejected session still sends')
check(stale.document.snapshot().equals(before), 'Rejection discarded content')
live.receive(rejected)
check(live.state.status === 'active', 'Unrelated rejection changed another writer')
stale.document.edit(tx => tx.increment(['count'], 2n))
check(stale.document.get(['count'])?.toJS() === 3n, 'Recovery blocked local work')
stale.close()
live.close()

// Writing the root into a descendant stores a finite copy of the prior tree.
const doc = Document.create({ title: 'Draft', archive: [], items: ['A', 'B', 'C'] })
const frozen = doc.snapshot()
doc.edit(tx => tx.list(['archive']).insert(0, [tx.get([])!]))
check(doc.get(['archive', 0])!.equals(frozen), 'Root copy changed source content')
doc.edit(tx => tx.list(['items']).move(0, 2))
check(
  JSON.stringify(doc.get(['items'])?.toJS()) === '["B","C","A"]',
  'Move index is not post-removal',
)
doc.edit(tx => {
  tx.set(['title'], 'Ready')
  tx.set(['approved'], true)
  tx.list(['items']).insert(3, ['D'])
})
check(doc.get(['title'])?.toJS() === 'Ready' && doc.has(['approved']), 'Map Set semantics changed')
for (const edit of [
  () =>
    doc.edit(tx => {
      tx.set(['title'], 'invalid')
      tx.set(['items', 4], 'E')
    }),
  () => doc.edit(tx => tx.set(['missing', 'child'], true)),
  () => doc.edit(tx => tx.list(['items']).delete(3, 2)),
]) {
  const beforeFailure = doc.snapshot()
  const version = doc.version
  let failed = false
  try {
    edit()
  } catch (error) {
    if (!(error instanceof CollaError)) throw error
    failed = true
  }
  check(failed, 'Invalid Map/List edit unexpectedly succeeded')
  check(
    doc.snapshot().equals(beforeFailure) && doc.version === version,
    'Failed edit committed partial state',
  )
}
doc.close()
console.log('Rejection recovery, root Copy, ListMove coordinates and Map/List boundaries passed')
