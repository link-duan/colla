// Sidebar presentation. Required content is checked independently in topics.json.
export const sidebar = {
  '/docs/': [
    {
      text: 'Getting started',
      collapsed: false,
      items: [
        {
          text: 'Introduction',
          link: '/docs/getting-started/',
        },
        {
          text: 'Installation',
          link: '/docs/getting-started/installation',
        },
        {
          text: 'Tutorial',
          link: '/docs/getting-started/tutorial',
        },
      ],
    },
    {
      text: 'Core',
      collapsed: false,
      items: [
        {
          text: 'Core concepts',
          link: '/docs/core/',
        },
        {
          text: 'Values',
          link: '/docs/core/values',
        },
        {
          text: 'Paths',
          link: '/docs/core/paths',
        },
        {
          text: 'Text and RichText',
          link: '/docs/core/text',
        },
        {
          text: 'Positions',
          link: '/docs/core/positions',
        },
        {
          text: 'Changes',
          link: '/docs/core/changes',
        },
        {
          text: 'Change algebra',
          link: '/docs/core/algebra',
        },
        {
          text: 'Concurrent edits',
          link: '/docs/core/concurrency',
        },
      ],
    },
    {
      text: 'Editing',
      collapsed: false,
      items: [
        {
          text: 'Document and snapshots',
          link: '/docs/editing/',
        },
        {
          text: 'Transactions and editors',
          link: '/docs/editing/transactions',
        },
        {
          text: 'Map and List editing',
          link: '/docs/editing/maps-lists',
        },
        {
          text: 'Text and RichText editing',
          link: '/docs/editing/text',
        },
        {
          text: 'Edit results and steps',
          link: '/docs/editing/results',
        },
        {
          text: 'Subscriptions and events',
          link: '/docs/editing/events',
        },
        {
          text: 'Runtime lifecycle',
          link: '/docs/editing/lifecycle',
        },
        {
          text: 'Editor integration',
          link: '/docs/editing/editor-integration',
        },
      ],
    },
    {
      text: 'History',
      collapsed: false,
      items: [
        {
          text: 'Undo and redo',
          link: '/docs/history/',
        },
        {
          text: 'Grouping and capacity',
          link: '/docs/history/grouping',
        },
        {
          text: 'Remote changes and rebasing',
          link: '/docs/history/rebasing',
        },
        {
          text: 'Checkpoints and restoration',
          link: '/docs/history/checkpoints',
        },
      ],
    },
    {
      text: 'Sync',
      collapsed: false,
      items: [
        {
          text: 'Synchronization overview',
          link: '/docs/sync/',
        },
        {
          text: 'SyncSession',
          link: '/docs/sync/session',
        },
        {
          text: 'Submissions and commits',
          link: '/docs/sync/submissions',
        },
        {
          text: 'Authority',
          link: '/docs/sync/authority',
        },
        {
          text: 'Concurrency and conflicts',
          link: '/docs/sync/concurrency',
        },
        {
          text: 'Retries and revision gaps',
          link: '/docs/sync/retries',
        },
        {
          text: 'Recovery',
          link: '/docs/sync/recovery',
        },
      ],
    },
    {
      text: 'Examples',
      collapsed: false,
      items: [
        {
          text: 'ListMove',
          link: '/docs/examples/list-move',
        },
        {
          text: 'Two-client synchronization',
          link: '/docs/examples/sync',
        },
        {
          text: 'Collaborative undo',
          link: '/docs/examples/history',
        },
        {
          text: 'Editor adapter',
          link: '/docs/examples/editor',
        },
        {
          text: 'Rust',
          link: '/docs/examples/rust',
        },
      ],
    },
    {
      text: 'Production',
      collapsed: false,
      items: [
        {
          text: 'Persistence and restart',
          link: '/docs/production/persistence',
        },
        {
          text: 'Transport and authentication',
          link: '/docs/production/transport',
        },
        {
          text: 'Errors and resource limits',
          link: '/docs/production/errors-limits',
        },
        {
          text: 'Integration testing',
          link: '/docs/production/testing',
        },
      ],
    },
  ],
  '/reference/': [
    {
      text: 'Reference',
      items: [
        {
          text: 'JavaScript API',
          link: '/reference/javascript',
        },
        {
          text: 'Rust API',
          link: '/reference/rust',
        },
        {
          text: 'Protocol and encoding',
          link: '/reference/protocol',
        },
        {
          text: 'Glossary and errors',
          link: '/reference/glossary',
        },
      ],
    },
  ],
}
