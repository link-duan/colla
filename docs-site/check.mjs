import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, extname, join, normalize, relative, resolve, sep } from 'node:path'

const root = new URL('.', import.meta.url).pathname
const configPath = join(root, '.vitepress', 'config.mts')
const ignoredDirectories = new Set(['.vitepress', 'node_modules'])
const markdown = []

function visit(directory) {
  for (const entry of readdirSync(directory)) {
    if (ignoredDirectories.has(entry)) continue
    const path = join(directory, entry)
    if (statSync(path).isDirectory()) visit(path)
    else if (entry.endsWith('.md')) markdown.push(path)
  }
}

visit(root)
const relativePages = new Set(markdown.map(path => relative(root, path)))
const errors = []

function routeToPageCandidates(route) {
  const clean = route.replace(/^\/colla(?:\/|$)/, '/').replace(/\/$/, '')
  const page = clean.replace(/^\//, '')
  return [join(root, `${page}.md`), join(root, page, 'index.md')]
}

function routeExists(route) {
  return routeToPageCandidates(route).some(existsSync)
}

function configuredRoutes() {
  const source = readFileSync(configPath, 'utf8')
  const routes = new Set()
  const pattern = /\blink\s*:\s*["'](\/(?:docs|reference)(?:\/[A-Za-z0-9_.-]+)+\/??)["']/g
  for (const match of source.matchAll(pattern)) routes.add(match[1].replace(/\/$/, ''))
  return [...routes]
}

const primaryRoutes = configuredRoutes()
for (const route of primaryRoutes) {
  if (!routeExists(route)) errors.push(`missing sidebar route: ${route}`)
}

const legacyPages = [
  'guide/quick-start.md',
  'guide/javascript.md',
  'guide/rust.md',
  'concepts/data-model.md',
  'concepts/document-model.md',
  'docs/building.md',
  'docs/core-concepts.md',
  'docs/document-workflow.md',
  'docs/ot-model.md',
]
for (const page of legacyPages) {
  if (!relativePages.has(page)) errors.push(`missing legacy route: ${page}`)
}

for (const route of primaryRoutes) {
  const file = routeToPageCandidates(route).find(existsSync)
  if (file === undefined) continue
  const lines = readFileSync(file, 'utf8').trim().split(/\r?\n/).length
  if (lines < 35) errors.push(`sidebar page is too short (${lines} lines): ${relative(root, file)}`)
}

const staleDirectories = ['start', 'build', 'understand', 'spec']
for (const directory of staleDirectories) {
  const path = join(root, directory)
  if (!existsSync(path)) continue
  const stale = []
  const walk = current => {
    for (const entry of readdirSync(current)) {
      const child = join(current, entry)
      if (statSync(child).isDirectory()) walk(child)
      else if (extname(child) === '.md') stale.push(child)
    }
  }
  walk(path)
  if (stale.length) errors.push(`stale split pages remain under ${directory}/`)
}

function routeFromPath(pathname) {
  const relativePath = relative(root, pathname).split(sep).join('/')
  if (relativePath.endsWith('.md')) {
    const withoutExtension = relativePath.slice(0, -3)
    if (withoutExtension.endsWith('/index')) {
      const parent = withoutExtension.slice(0, -6)
      return `/colla/${parent}`.replace(/\/+/g, '/')
    }
    return `/colla/${withoutExtension}`.replace(/\/+/g, '/')
  }
  if (existsSync(`${pathname}.md`)) {
    return `/colla/${relativePath}`.replace(/\/+/g, '/')
  }
  if (existsSync(join(pathname, 'index.md'))) {
    const parent = relativePath.replace(/\/$/, '')
    return `/colla/${parent}`.replace(/\/+/g, '/')
  }
  return null
}

function resolveMarkdownTarget(sourceFile, target) {
  if (!target || target.startsWith('#')) return null

  // VitePress adds the configured base to Markdown links, but raw HTML
  // attributes are emitted as written. Treat site-root docs/reference links
  // as base-relative so the checker catches links that would 404 on GitHub
  // Pages when `base` is `/colla/`.
  if (target.startsWith('/docs/') || target.startsWith('/reference/')) {
    return `/colla${target.split('#')[0]}`
  }

  if (/^(?:[A-Za-z][A-Za-z0-9+.-]*:|\/\/)/.test(target)) {
    if (target.startsWith('/colla/')) return target.split('#')[0]
    return null
  }

  const [pathname] = target.split('#')
  if (!pathname || pathname.startsWith('mailto:')) return null
  if (!pathname.endsWith('.md') && !pathname.endsWith('/')) {
    // VitePress clean URLs are commonly written without an extension. Only
    // inspect path-like targets; ordinary fragment or asset links are ignored.
    if (!pathname.includes('/')) return null
  }
  const resolved = normalize(resolve(dirname(sourceFile), pathname))
  if (!resolved.startsWith(root)) return null
  return routeFromPath(resolved)
}

function checkLink(sourceFile, target) {
  const route = resolveMarkdownTarget(sourceFile, target)
  if (route !== null && !routeExists(route)) errors.push(`${relative(root, sourceFile)} -> ${target}`)
}

for (const file of markdown) {
  const source = readFileSync(file, 'utf8')
  for (const match of source.matchAll(/!?\[[^\]]*\]\(([^)\s]+)(?:\s+["'][^)]*["'])?\)/g)) {
    checkLink(file, match[1])
  }
  for (const match of source.matchAll(/\bhref=["']([^"']+)["']/g)) {
    checkLink(file, match[1])
  }
}

if (errors.length) {
  console.error('Documentation structure/link checks failed:')
  for (const error of errors) console.error(`- ${error}`)
  process.exit(1)
}

console.log(`Checked ${markdown.length} pages, ${primaryRoutes.length} sidebar routes, and the Wiki Tree structure.`)
