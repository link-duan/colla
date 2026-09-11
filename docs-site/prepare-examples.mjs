import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('.', import.meta.url))
const generated = join(root, '.examples-generated')
rmSync(generated, { recursive: true, force: true })
mkdirSync(generated)
function markdown(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const file = join(directory, entry.name)
    return entry.isDirectory() ? markdown(file) : file.endsWith('.md') ? [file] : []
  })
}
const sources = [...markdown(join(root, 'docs')), join(root, '../README.md'), join(root, '../packages/core/README.md')]
let count = 0
for (const source of sources) {
  const snippets = [...readFileSync(source, 'utf8').matchAll(/^```ts\n([^]*?)^```/gm)]
  for (const [index, match] of snippets.entries()) {
    if (!/^import /m.test(match[1])) throw new Error(`Runnable snippet must include imports: ${source}`)
    const name = relative(root, source).replace(/[^a-zA-Z0-9-]/g, '-')
    writeFileSync(join(generated, `${name}-${index}.ts`), match[1])
    count++
  }
}
console.log(`Prepared ${count} standalone TypeScript documentation snippets.`)
