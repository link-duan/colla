import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { dirname, extname, join, relative, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { createMarkdownRenderer } from 'vitepress'

const defaultRoot = fileURLToPath(new URL('.', import.meta.url))
const ignored = new Set(['.vitepress', 'node_modules', '.examples-dist', '.examples-generated'])
function markdownFiles(root, directory = root) {
  return readdirSync(directory).flatMap(name => {
    if (ignored.has(name)) return []
    const path = join(directory, name)
    return statSync(path).isDirectory() ? markdownFiles(root, path) : name.endsWith('.md') ? [path] : []
  })
}
function linksInSidebar(sidebar) {
  const links = []
  const visit = items => items.forEach(item => {
    if (item.link) links.push(item.link)
    if (item.items) visit(item.items)
  })
  Object.values(sidebar).forEach(visit)
  return links
}

// Match the full-file VitePress include form used by this site. Reject other forms
// rather than checking a different document from the one VitePress will publish.
function expandIncludes(file, ancestors = []) {
  if (ancestors.includes(file)) throw new Error(`cyclic Markdown include: ${file}`)
  return readFileSync(file, 'utf8').replace(/<!--\s*@include:\s*(.*?)\s*-->/g, (_, name) => {
    if (!name || /[#{}]/.test(name) || name.startsWith('@')) {
      throw new Error(`unsupported Markdown include: ${name}; use a full relative file path`)
    }
    const target = resolve(dirname(file), name)
    if (!existsSync(target)) throw new Error(`missing Markdown include: ${name}`)
    return expandIncludes(target, [...ancestors, file]).replace(/^---\r?\n[^]*?\r?\n---(?:\r?\n|$)/, '')
  })
}

export async function checkDocs({ root = defaultRoot, sidebar, required } = {}) {
  root = resolve(root)
  if (!sidebar) ({ sidebar } = await import(pathToFileURL(join(root, '.vitepress/navigation.mjs')).href))
  required ??= JSON.parse(readFileSync(join(root, 'topics.json'), 'utf8'))
  const errors = []
  const files = markdownFiles(root)
  const fileSet = new Set(files)
  const renderer = await createMarkdownRenderer(root, { languages: ['ts', 'rust', 'sh', 'toml'] }, '/colla/')
  const pages = new Map()
  for (const file of files) {
    let source
    try { source = expandIncludes(file) } catch (error) {
      errors.push(`${relative(root, file)}: ${error.message}`)
      source = readFileSync(file, 'utf8')
    }
    const env = { path: file, relativePath: relative(root, file) }
    // Render with the site's own Markdown engine: heading IDs and snippet inclusion
    // must follow VitePress, including duplicate and explicit heading anchors.
    let html
    try { html = renderer.render(source, env) } catch (error) {
      errors.push(`${relative(root, file)}: ${error.message}`)
      html = ''
    }
    const ids = new Set([...html.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]))
    const links = [...html.matchAll(/\b(?:href|src)="([^"]+)"/g)].map(match => match[1].replaceAll('&amp;', '&'))
    for (const match of source.matchAll(/^\s+link:\s*["']?([^\s"']+)/gm)) links.push(match[1])
    pages.set(file, { ids, links })
    if (/This topic is maintained in the \[current guide\]|# Colla 0\.4 documentation|\bTODO\b|\bTBD\b/.test(source)) {
      errors.push(`placeholder page: ${relative(root, file)}`)
    }
    // Snippet source paths are not emitted as links in HTML.
    for (const match of source.matchAll(/^<<<\s+([^\s{]+)/gm)) {
      if (!existsSync(resolve(dirname(file), match[1].split('#')[0]))) errors.push(`missing snippet: ${relative(root, file)} -> ${match[1]}`)
    }
  }
  function targetFile(sourceFile, href) {
    if (/^(?:[a-z][a-z\d+.-]*:|\/\/)/i.test(href)) return null
    const [pathname, fragment] = href.split('#')
    let path
    try {
      const decoded = decodeURIComponent(pathname.split('?')[0])
      path = decoded ? decoded.startsWith('/')
        ? resolve(root, '.' + decoded.replace(/^\/colla(?=\/|$)/, ''))
        : resolve(dirname(sourceFile), decoded)
        : sourceFile
    } catch { return { error: 'malformed URL' } }
    if (path !== root && !path.startsWith(root + '/')) return { error: 'link escapes site' }
    const stem = path.replace(/\.(?:html|md)$/, '')
    const candidates = [path, stem + '.md', join(stem, 'index.md')]
    const page = candidates.find(candidate => fileSet.has(candidate))
    if (page) return { page, fragment }
    // Non-document local assets may come from public/.
    if (extname(path) && extname(path) !== '.html' && extname(path) !== '.md') {
      const publicPath = join(root, 'public', relative(root, path))
      if ([path, publicPath].some(candidate => existsSync(candidate) && statSync(candidate).isFile())) return null
    }
    return { error: 'missing target' }
  }
  const configPath = join(root, '.vitepress/config.mts')
  if (existsSync(configPath)) {
    for (const match of readFileSync(configPath, 'utf8').matchAll(/\blink:\s*["'](\/[^"']+)["']/g)) {
      const target = targetFile(join(root, 'index.md'), match[1])
      if (target?.error) errors.push(`missing configured navigation route: ${match[1]}`)
    }
  }
  const sidebarPages = new Set()
  for (const route of linksInSidebar(sidebar)) {
    const target = targetFile(join(root, 'index.md'), route)
    if (!target?.page) errors.push(`missing sidebar route: ${route}`)
    else sidebarPages.add(target.page)
  }
  for (const name of required) {
    const file = join(root, name)
    if (!fileSet.has(file)) errors.push(`missing required topic: ${name}`)
    if (!sidebarPages.has(file)) errors.push(`required topic missing from navigation: ${name}`)
  }
  for (const [file, page] of pages) {
    const name = relative(root, file)
    if (name !== 'index.md') {
      if (!required.includes(name)) errors.push(`unregistered topic: ${name}`)
      if (!sidebarPages.has(file)) errors.push(`orphan page: ${name}`)

    }
    for (const href of page.links) {
      const target = targetFile(file, href)
      if (target?.error) errors.push(`${name} -> ${href}: ${target.error}`)
      else if (target?.page && target.fragment) {
        let anchor
        try { anchor = decodeURIComponent(target.fragment) } catch { anchor = target.fragment }
        if (!pages.get(target.page)?.ids.has(anchor)) errors.push(`${name} -> ${href}: missing anchor`)
      }
    }
  }
  return { errors, pageCount: files.length, sidebarCount: sidebarPages.size }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = await checkDocs()
  if (result.errors.length) {
    console.error('Documentation checks failed:\n' + result.errors.map(error => `- ${error}`).join('\n'))
    process.exitCode = 1
  } else console.log(`Checked ${result.pageCount} pages, ${result.sidebarCount} sidebar topics, links, anchors and snippets.`)
}
