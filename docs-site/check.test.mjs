import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import assert from 'node:assert/strict'
import test from 'node:test'
import { checkDocs } from './check.mjs'

const prose = 'A document snapshot preserves content and identities across edits.'
async function fixture({ content, sidebar = true, extra } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'colla-docs-check-'))
  mkdirSync(join(root, 'docs'))
  writeFileSync(join(root, 'index.md'), '# Home\n\n[Start](/docs/start)\n')
  writeFileSync(join(root, 'docs/start.md'), content ?? `# Start\n\n## Read\n\n${prose}\n\n## Edit\n\n[Read](#read)\n`)
  if (extra) writeFileSync(join(root, 'docs/extra.md'), extra)
  try {
    return (await checkDocs({ root, required: ['docs/start.md'], sidebar: { '/docs/': sidebar ? [{ link: '/docs/start' }] : [] } })).errors
  } finally { rmSync(root, { recursive: true, force: true }) }
}
test('complete topic and local anchor pass', async () => assert.deepEqual(await fixture(), []))
test('removing navigation does not reduce required coverage', async () => {
  assert.ok((await fixture({ sidebar: false })).some(error => error.includes('missing from navigation')))
})
test('placeholder topics fail', async () => {
  const errors = await fixture({ content: '# Colla 0.4 documentation\n\nThis topic is maintained in the [current guide](/docs/start).' })
  assert.ok(errors.some(error => error.includes('placeholder')))
})
test('broken relative paths, root paths and anchors fail', async () => {
  const errors = await fixture({ content: `# Start\n\n## Read\n${prose}\n## Edit\n[Relative](./missing)\n[Root](/docs/missing)\n[Anchor](./start#absent)` })
  for (const target of ['missing', 'absent']) assert.ok(errors.some(error => error.includes(target)), target)
  assert.equal(errors.filter(error => error.includes('missing target')).length, 2)
  assert.ok(errors.some(error => error.includes('missing anchor')))
})
test('unregistered pages and missing snippets fail', async () => {
  const errors = await fixture({ extra: `# Extra\n\n## Read\n${prose}\n## Edit\n<<< ./missing.ts` })
  assert.ok(errors.some(error => error.includes('unregistered topic')))
  assert.ok(errors.some(error => error.includes('orphan page')))
  assert.ok(errors.some(error => error.includes('missing snippet')))
})
test('explicit, Unicode and duplicate heading IDs use VitePress semantics', async () => {
  assert.deepEqual(await fixture({ content: `# Start\n\n## Read {#custom}\n${prose}\n## 编辑\n## Same\n## Same\n[Custom](#custom) [Unicode](#%E7%BC%96%E8%BE%91) [Duplicate](#same-1)` }), [])
})

test('home-style frontmatter actions are checked', async () => {
  const errors = await fixture({ content: `---
hero:
  actions:
    - text: Start
      link: /docs/removed
---
# Start

## Read
${prose}
## Edit
` })
  assert.ok(errors.some(error => error.includes('/docs/removed')))
})
test('missing required topic fails independently of navigation', async () => {
  const root = mkdtempSync(join(tmpdir(), 'colla-docs-required-'))
  writeFileSync(join(root, 'index.md'), '# Home')
  try {
    const { errors } = await checkDocs({ root, required: ['docs/removed.md'], sidebar: {} })
    assert.ok(errors.includes('missing required topic: docs/removed.md'))
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('a concise single-topic article needs no minimum word or section count', async () => {
  assert.deepEqual(await fixture({ content: '# Snapshot\n\nA frozen Value observes only its own content.' }), [])
})
test('included Markdown supplies checked links and heading anchors', async () => {
  const root = mkdtempSync(join(tmpdir(), 'colla-docs-include-'))
  mkdirSync(join(root, 'site/docs'), { recursive: true })
  const source = join(root, 'contract.md')
  const page = join(root, 'site/docs/start.md')
  writeFileSync(page, '<!--@include: ../../contract.md-->')
  const options = { root: join(root, 'site'), required: ['docs/start.md'], sidebar: { '/docs/': [{ link: '/docs/start' }] } }
  try {
    writeFileSync(source, '# Protocol\n\n## Fields\n\n[Fields](#fields)')
    assert.deepEqual((await checkDocs(options)).errors, [])
    writeFileSync(source, '# Protocol\n\n[Missing](#absent)\n[Broken](/docs/gone)')
    const errors = (await checkDocs(options)).errors
    assert.ok(errors.some(error => error.includes('missing anchor')))
    assert.ok(errors.some(error => error.includes('missing target')))
    writeFileSync(source, '<!--@include: ./site/docs/start.md-->')
    assert.ok((await checkDocs(options)).errors.some(error => error.includes('cyclic Markdown include')))
    rmSync(source)
    assert.ok((await checkDocs(options)).errors.some(error => error.includes('missing Markdown include')))
  } finally { rmSync(root, { recursive: true, force: true }) }
})
