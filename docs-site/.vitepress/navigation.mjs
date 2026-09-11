// Sidebar presentation. Required content is checked independently in topics.json.
export const sidebar = {
  "/docs/": [
    {
      "text": "Getting started",
      "link": "/docs/getting-started/"
    },
    {
      "text": "Core",
      "collapsed": false,
      "items": [
        {
          "text": "Values and types",
          "link": "/docs/core/values"
        },
        {
          "text": "Element identity and paths",
          "link": "/docs/core/identity"
        },
        {
          "text": "Text",
          "link": "/docs/core/text"
        },
        {
          "text": "RichText",
          "link": "/docs/core/richtext"
        },
        {
          "text": "Text coordinates",
          "link": "/docs/core/coordinates"
        },
        {
          "text": "Move, Copy and Set",
          "link": "/docs/core/move-copy-set"
        },
        {
          "text": "References",
          "link": "/docs/core/references"
        },
        {
          "text": "Changes and OT algebra",
          "link": "/docs/core/changes"
        }
      ]
    },
    {
      "text": "Editing",
      "collapsed": false,
      "items": [
        {
          "text": "Document and snapshots",
          "link": "/docs/editing/"
        },
        {
          "text": "Transactions and editors",
          "link": "/docs/editing/transactions"
        },
        {
          "text": "Map and List editing",
          "link": "/docs/editing/maps-lists"
        },
        {
          "text": "Edit results and steps",
          "link": "/docs/editing/results"
        },
        {
          "text": "Subscriptions and events",
          "link": "/docs/editing/events"
        },
        {
          "text": "Runtime lifecycle",
          "link": "/docs/editing/lifecycle"
        },
        {
          "text": "Editor integration",
          "link": "/docs/editing/editor-integration"
        }
      ]
    },
    {
      "text": "History",
      "collapsed": false,
      "items": [
        {
          "text": "Undo and redo",
          "link": "/docs/history/"
        },
        {
          "text": "Grouping and capacity",
          "link": "/docs/history/grouping"
        },
        {
          "text": "Remote changes and rebasing",
          "link": "/docs/history/rebasing"
        },
        {
          "text": "Checkpoints and restoration",
          "link": "/docs/history/checkpoints"
        }
      ]
    },
    {
      "text": "Sync",
      "collapsed": false,
      "items": [
        {
          "text": "Synchronization overview",
          "link": "/docs/sync/"
        },
        {
          "text": "SyncSession",
          "link": "/docs/sync/session"
        },
        {
          "text": "Submissions and commits",
          "link": "/docs/sync/submissions"
        },
        {
          "text": "Authority",
          "link": "/docs/sync/authority"
        },
        {
          "text": "Concurrency and conflicts",
          "link": "/docs/sync/concurrency"
        },
        {
          "text": "Retries and revision gaps",
          "link": "/docs/sync/retries"
        },
        {
          "text": "Recovery",
          "link": "/docs/sync/recovery"
        }
      ]
    },
    {
      "text": "Examples",
      "collapsed": false,
      "items": [
        {
          "text": "Move and Ref",
          "link": "/docs/examples/move-ref"
        },
        {
          "text": "Two-client synchronization",
          "link": "/docs/examples/sync"
        },
        {
          "text": "Collaborative undo",
          "link": "/docs/examples/history"
        },
        {
          "text": "Editor adapter",
          "link": "/docs/examples/editor"
        },
        {
          "text": "Rust",
          "link": "/docs/examples/rust"
        }
      ]
    },
    {
      "text": "Production",
      "collapsed": false,
      "items": [
        {
          "text": "Persistence and restart",
          "link": "/docs/production/persistence"
        },
        {
          "text": "Transport and authentication",
          "link": "/docs/production/transport"
        },
        {
          "text": "Errors and resource limits",
          "link": "/docs/production/errors-limits"
        },
        {
          "text": "Integration testing",
          "link": "/docs/production/testing"
        }
      ]
    }
  ],
  "/reference/": [
    {
      "text": "Reference",
      "items": [
        {
          "text": "JavaScript API",
          "link": "/reference/javascript"
        },
        {
          "text": "Rust API",
          "link": "/reference/rust"
        },
        {
          "text": "Protocol and encoding",
          "link": "/reference/protocol"
        },
        {
          "text": "Glossary and errors",
          "link": "/reference/glossary"
        }
      ]
    }
  ]
}
