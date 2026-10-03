import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'
import * as colla from '../dist/node.js'
const bytes = hex => Uint8Array.from(Buffer.from(hex, 'hex'))
const hex = value => Buffer.from(value.encode()).toString('hex')
const fixtures = JSON.parse(
  await readFile(new URL('../../../golden/fixtures.json', import.meta.url), 'utf8'),
)
for (const fixture of fixtures)
  test(`shared fixture: ${fixture.name}`, () => {
    if (fixture.kind === 'codec') {
      const type = [
        null,
        colla.Value,
        colla.Change,
        colla.SyncSnapshot,
        colla.Submission,
        colla.ServerMessage,
        colla.SessionCheckpoint,
        colla.HistoryCheckpoint,
        colla.AuthorityCheckpoint,
      ][fixture.type]
      const encoded = bytes(fixture.bytes)
      assert.equal(new DataView(encoded.buffer).getUint16(5, true), 1)
      assert.equal(hex(type.decode(encoded)), fixture.bytes)
      for (const version of [0, 2]) {
        const invalid = encoded.slice()
        new DataView(invalid.buffer).setUint16(5, version, true)
        assert.throws(() => type.decode(invalid), { code: 'invalid_encoding' })
      }
      return
    }
    const base = colla.Value.decode(bytes(fixture.base))
    const left = colla.Change.decode(bytes(fixture.left)),
      right = colla.Change.decode(bytes(fixture.right))
    const [l, r] = colla.transform(base, left, right, { priority: 'left' })
    assert.equal(hex(l), fixture.leftAfterRight)
    assert.equal(hex(r), fixture.rightAfterLeft)
    const after = colla.apply(base, left)
    assert.equal(hex(after), fixture.after)
    assert.equal(hex(colla.invert(base, left)), fixture.inverse)
    assert.equal(hex(colla.compose(base, left, r)), fixture.composed)
    assert.equal(hex(colla.apply(after, r)), fixture.merged)
    assert.ok(colla.apply(colla.apply(base, right), l).equals(colla.apply(after, r)))
  })
