import { readFileSync } from 'node:fs'
import { defineConfig } from 'vitepress'

const packageJson = JSON.parse(
  readFileSync(new URL('../../packages/core/package.json', import.meta.url), 'utf8'),
) as { version: string }

export default defineConfig({
  title: 'Colla',
  titleTemplate: ':title · Colla',
  description: 'Operational Transformation for structured documents',
  base: '/colla/',
  cleanUrls: true,
  lastUpdated: true,
  srcExclude: ['**/internal/**', '**/agents/**', '**/adr/**'],
  appearance: 'dark',
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/colla/favicon.svg' }],
    ['meta', { name: 'theme-color', content: '#0a0a0a' }],
    ['meta', { name: 'color-scheme', content: 'dark light' }],
  ],
  themeConfig: {
    logo: '/colla-logo.svg',
    siteTitle: 'Colla',
    nav: [
      { text: 'Docs', link: '/docs/getting-started', activeMatch: '/docs/' },
      { text: 'Reference', link: '/reference/javascript', activeMatch: '/reference/' },
      { text: `v${packageJson.version}`, link: 'https://github.com/link-duan/colla/blob/master/CHANGELOG.md', noIcon: true },
    ],
    sidebar: {
      '/docs/': [
        { text: 'Getting started', items: [
          { text: 'Overview', link: '/docs/getting-started' },
          { text: 'Install and first edit', link: '/docs/getting-started/install' },
        ] },
        { text: 'Data model', items: [
          { text: 'Values', link: '/docs/core/values' },
          { text: 'Changes', link: '/docs/core/changes' },
          { text: 'Sequences', items: [
            { text: 'Text', link: '/docs/core/text' },
            { text: 'RichText', link: '/docs/core/richtext' },
          ] },
          { text: 'Coordinates and paths', link: '/docs/core/coordinates' },
        ] },
        { text: 'Document state', items: [
          { text: 'Overview', link: '/docs/document/' },
          { text: 'State', link: '/docs/document/state' },
          { text: 'Lifecycle', link: '/docs/document/lifecycle' },
          { text: 'Snapshot and Update', link: '/docs/document/snapshot-update' },
          { text: 'Events and lifecycle', link: '/docs/document/events-lifecycle' },
          { text: 'Local and remote updates', link: '/docs/document/local-remote' },
          { text: 'Envelopes', link: '/docs/document/envelopes' },
          { text: 'Editor integration', link: '/docs/document/editor-integration' },
        ] },
        { text: 'OT and synchronization', items: [
          { text: 'Overview', link: '/docs/ot/' },
          { text: 'Apply, Compose, and Invert', link: '/docs/ot/algebra' },
          { text: 'Transform and rebasing', link: '/docs/ot/transform-rebase' },
          { text: 'Concurrency', link: '/docs/ot/concurrency' },
          { text: 'Protocol boundaries', link: '/docs/ot/protocol-boundaries' },
        ] },
        { text: 'Examples', items: [
          { text: 'Overview', link: '/docs/examples/' },
          { text: 'JavaScript', items: [
            { text: 'Document synchronization', link: '/docs/examples/javascript-document' },
            { text: 'Immutable Value and Change', link: '/docs/examples/javascript-core' },
          ] },
          { text: 'Rust', link: '/docs/examples/rust' },
          { text: 'Editor integration', link: '/docs/examples/editor-integration' },
        ] },
        { text: 'Production', items: [
          { text: 'Overview', link: '/docs/production/' },
          { text: 'Persistence and recovery', link: '/docs/production/persistence' },
          { text: 'Sync protocol', link: '/docs/production/sync-protocol' },
          { text: 'Security and limits', link: '/docs/production/security' },
          { text: 'Testing and guarantees', link: '/docs/production/testing' },
        ] },
      ],
      '/reference/': [{ text: 'Reference', items: [
        { text: 'JavaScript API', link: '/reference/javascript' },
        { text: 'Rust API', link: '/reference/rust' },
        { text: 'Protocol reference', link: '/reference/protocol' },
        { text: 'Glossary and errors', link: '/reference/glossary' },
      ] }],
    },
    outline: { level: [2, 3] },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/link-duan/colla' },
    ],
    search: { provider: 'local' },
    editLink: { pattern: 'https://github.com/link-duan/colla/edit/master/docs-site/:path' },
    footer: {
      message: `Colla v${packageJson.version} · Released under the MIT License.`,
      copyright: 'Copyright © Colla contributors',
    },
  },
})
