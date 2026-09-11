import assert from "node:assert/strict"
import { Authority, Document, History, SyncSession } from "../dist/node.js"
import { initSync } from "../dist/internal/colla_wasm.js"
if (!globalThis.gc) throw Error("run with --expose-gc")
const wasm = initSync()
const nextTurn = () => new Promise(resolve => setImmediate(resolve))
async function collect() { for (let n = 0; n < 3; n++) { globalThis.gc(); await nextTurn() } }
const session = SyncSession.create({ clientId: "memory", snapshot: Authority.create({ documentId: "memory", value: { count: 0n } }).snapshot() })
const doc = session.document, id = doc.idAt(["count"])
History.attach(doc)
const samples = []
for (let batch = 0; batch < 30; batch++) {
  for (let n = 0; n < 200; n++) {
    doc.edit(tx => tx.increment(id, 1n))
    const temporary = Document.create(doc.snapshot())
    temporary.close()
  }
  await collect()
  samples.push({ heap: process.memoryUsage().heapUsed, wasm: wasm.memory.buffer.byteLength, checkpoint: session.checkpoint().encode().byteLength })
}
assert.equal(doc.get(id).toJS(), 6000n)
const warm = samples[9], last = samples.at(-1)
assert.ok(last.heap - warm.heap < 8 * 1024 * 1024, "JS heap grows after warmup")
assert.ok(last.wasm - warm.wasm < 8 * 1024 * 1024, "Wasm memory grows after warmup")
assert.ok(last.checkpoint < 20000, "single in-flight + compact buffer/history should stay bounded")
session.close()
await collect()
console.log(JSON.stringify({ edits: 6000, warm, final: last, growth: { heap: last.heap - warm.heap, wasm: last.wasm - warm.wasm } }))
