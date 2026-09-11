import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import test from 'node:test'

const expected = {
  basic_edit: [
    'Path after move: Some([Key("to"), Index(0)])',
    'Ref still targets the same item: true',
    'Edited title: Text("Draft v2")',
    'Path after undo: Some([Key("from"), Index(0)])',
    'Restored title: Text("Draft")',
  ],
  binary_roundtrip: [
    'Value content and IDs preserved: true',
    'Authority snapshot preserved: true',
  ],
  collab_demo: [
    'Alice after restart: Int(1)',
    'Confirmed server revision: 1',
    'Confirmed server revision: 2',
    'Alice after sync: Int(3)',
    'Bob after sync: Int(3)',
  ],
}
for (const [name, lines] of Object.entries(expected)) {
  test(`${name}: documented Rust output`, () => {
    const result = spawnSync('cargo', ['run', '--quiet', '-p', 'colla', '--example', name], {
      cwd: new URL('../', import.meta.url),
      encoding: 'utf8',
    })
    assert.equal(result.status, 0, result.stderr || result.error?.message)
    assert.deepEqual(result.stdout.trim().split('\n'), lines)
  })
}
