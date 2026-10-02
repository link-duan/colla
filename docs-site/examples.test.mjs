import assert from 'node:assert/strict'
import { readdirSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import test from 'node:test'

const expectedOutput = {
  'maps-lists': [
    "After insertion: [ 'draft', 'review', 'publish' ]",
    'Title: Ready',
    'Approved: true',
    'Temporary field exists: false',
    "Final steps: [ 'review', 'release' ]",
  ],
  'root-copy': [
    'Copy has a new ID: true',
    'Original title: Draft',
    'Archived title: Draft',
    'Archive inside copy: []',
  ],
  'session-restart': [
    'Before restart: 1n',
    'After restart: 1n',
    'Still awaiting confirmation: true',
  ],
  'first-edit': [
    "Before edit: Text { type: 'text', value: 'Draft' }",
    "After edit: Text { type: 'text', value: 'Draft v2' }",
  ],
  core: [
    "Left edit alone: Text { type: 'text', value: 'Say Hello' }",
    "Right edit alone: Text { type: 'text', value: 'Hello!' }",
    "Merged edits: Text { type: 'text', value: 'Say Hello!' }",
  ],
  sync: [
    'Before sync — Alice: 1n',
    'Before sync — Bob: 2n',
    'Confirmed server revision: 1n',
    'Confirmed server revision: 2n',
    'After sync — Alice: 3n',
    'After sync — Bob: 3n',
  ],
  history: [
    'Before undo — both contributions: 11n',
    'After undo — Bob’s contribution remains: 10n',
    'After redo — Alice’s contribution restored: 11n',
  ],
  'move-ref': [
    "Path before move: [ 'todo', 0 ]",
    "Path after move: [ 'done', 0 ]",
    'Ref still targets the same task: true',
  ],
  editor: [
    'Selected UTF-16 range: [1, 3) — one emoji, two code units',
    'Edit operations in Unicode scalars: [{"type":"retain","length":1},{"type":"insert","text":"🐬"},{"type":"delete","length":1}]',
    "Rendered text: Text { type: 'text', value: 'A🐬B' }",
  ],
}

function run(path) {
  const result = spawnSync(process.execPath, [new URL(path, import.meta.url).pathname], {
    encoding: 'utf8',
  })
  assert.equal(result.status, 0, result.stderr || result.error?.message)
  assert.equal(result.stderr, '', 'Examples should not hide listener errors')
  return result.stdout.trim().split('\n')
}

for (const name of readdirSync(new URL('./examples/', import.meta.url))) {
  if (!name.endsWith('.ts')) continue
  const slug = name.slice(0, -3)
  test(`${slug}: documented output`, () => {
    assert.ok(expectedOutput[slug], 'Define the expected output for each displayed example')
    assert.deepEqual(run(`./.examples-dist/examples/${slug}.js`), expectedOutput[slug])
  })
}

for (const name of readdirSync(new URL('./tests/examples/', import.meta.url))) {
  if (!name.endsWith('.ts')) continue
  test(`${name}: integration regressions`, () => {
    run(`./.examples-dist/tests/examples/${name.slice(0, -3)}.js`)
  })
}

for (const name of readdirSync(new URL('./.examples-generated/', import.meta.url))) {
  if (!name.endsWith('.ts')) continue
  test(`${name}: standalone documentation snippet`, () => {
    run(`./.examples-dist/.examples-generated/${name.slice(0, -3)}.js`)
  })
}
