import { sidebar } from './navigation.mjs'
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
    logo: {
      light: '/logo-light.svg',
      dark: '/logo-dark.svg',
    },
    siteTitle: 'Colla',
    nav: [
      { text: 'Docs', link: '/docs/getting-started/', activeMatch: '/docs/' },
      { text: 'Reference', link: '/reference/javascript', activeMatch: '/reference/' },
      { text: `v${packageJson.version}`, link: 'https://github.com/link-duan/colla/blob/master/CHANGELOG.md', noIcon: true },
    ],
    sidebar,
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
