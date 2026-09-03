<script setup lang="ts">
import { ref } from 'vue'

interface Scenario {
  id: string
  title: string
  label: string
  alice: {
    title: string
    rev: string
    code: string
    visible: string
    highlight: string
  }
  bob: {
    title: string
    rev: string
    code: string
    visible: string
    highlight: string
  }
  transform: {
    fn: string
    time: string
    metric: string
  }
  result: {
    text: string
    rev: string
    hash: string
    note: string
  }
}

const scenarios: Scenario[] = [
  {
    id: 'concurrent-insert',
    title: 'Concurrent Inserts',
    label: '01 · Two Users Type At Once',
    alice: {
      title: 'Client A (Alice)',
      rev: 'rev: 1',
      code: 'doc.applyLocal(insert(0, "Fast "))',
      visible: 'Fast Colla',
      highlight: 'Fast '
    },
    bob: {
      title: 'Client B (Bob)',
      rev: 'rev: 1',
      code: 'doc.applyLocal(insert(5, " Engine"))',
      visible: 'Colla Engine',
      highlight: ' Engine'
    },
    transform: {
      fn: 'transformPair(chA, chB)',
      time: '0.02ms',
      metric: 'Deterministic Rebase'
    },
    result: {
      text: 'Fast Colla Engine',
      rev: 'rev: 3',
      hash: 'sha256:4f8a...c02b',
      note: 'Identical on both clients · Zero central locks'
    }
  },
  {
    id: 'format-and-edit',
    title: 'Format & Insert',
    label: '02 · Overlapping Formatting',
    alice: {
      title: 'Client A (Alice)',
      rev: 'rev: 1',
      code: 'doc.applyLocal(format(0, 5, { bold: true }))',
      visible: '<strong>Colla</strong>',
      highlight: 'bold'
    },
    bob: {
      title: 'Client B (Bob)',
      rev: 'rev: 1',
      code: 'doc.applyLocal(insert(5, " v2"))',
      visible: 'Colla v2',
      highlight: ' v2'
    },
    transform: {
      fn: 'transformPair(chA, chB)',
      time: '0.03ms',
      metric: 'Span Re-anchoring'
    },
    result: {
      text: '<strong>Colla</strong> v2',
      rev: 'rev: 3',
      hash: 'sha256:7e19...81da',
      note: 'RichText spans preserved · No attribute split anomalies'
    }
  },
  {
    id: 'undo-redo',
    title: 'Invertible Undo',
    label: '03 · Local Undo Stack',
    alice: {
      title: 'Client A (Alice)',
      rev: 'rev: 2',
      code: 'const inv = change.invert(snapshot)',
      visible: 'Reverted cleanly',
      highlight: 'Undo'
    },
    bob: {
      title: 'Client B (Bob)',
      rev: 'rev: 2',
      code: 'doc.applyRemote(invRebased)',
      visible: 'Live State Preserved',
      highlight: 'Synced'
    },
    transform: {
      fn: 'invert(change) ∘ rebase',
      time: '0.01ms',
      metric: 'Algebraic Inverse'
    },
    result: {
      text: 'Colla Engine',
      rev: 'rev: 4',
      hash: 'sha256:b39c...08ff',
      note: 'Undo commutes algebraically across remote mutations'
    }
  }
]

const activeIndex = ref(0)
const activeScenario = ref<Scenario>(scenarios[0])

function setScenario(index: number) {
  activeIndex.value = index
  activeScenario.value = scenarios[index]
}
</script>

<template>
  <div class="ot-workbench">
    <!-- Workbench Chrome Header -->
    <div class="workbench-chrome">
      <div class="chrome-controls" aria-hidden="true">
        <span class="control-dot dot-red"></span>
        <span class="control-dot dot-yellow"></span>
        <span class="control-dot dot-green"></span>
      </div>
      <div class="chrome-tabs" role="tablist" aria-label="OT Scenarios">
        <button
          v-for="(s, idx) in scenarios"
          :key="s.id"
          role="tab"
          :aria-selected="activeIndex === idx"
          :class="['chrome-tab', { active: activeIndex === idx }]"
          @click="setScenario(idx)"
        >
          <span class="tab-index">0{{ idx + 1 }}</span>
          <span class="tab-title">{{ s.title }}</span>
        </button>
      </div>
      <div class="chrome-badge">
        <span class="live-pulse" aria-hidden="true"></span>
        <span>Colla OT Core</span>
      </div>
    </div>

    <!-- Active Scenario Description Banner -->
    <div class="workbench-subbar">
      <span class="subbar-tag">SCENARIO</span>
      <span class="subbar-label">{{ activeScenario.label }}</span>
    </div>

    <!-- Dual Client Simulation Arena -->
    <div class="workbench-arena">
      <!-- Peer A (Alice) -->
      <div class="arena-peer peer-alice">
        <div class="peer-header">
          <div class="peer-identity">
            <span class="peer-dot alice-dot" aria-hidden="true"></span>
            <span class="peer-name">{{ activeScenario.alice.title }}</span>
          </div>
          <span class="peer-rev">{{ activeScenario.alice.rev }}</span>
        </div>
        <div class="peer-code-box">
          <code>{{ activeScenario.alice.code }}</code>
        </div>
        <div class="peer-state-box">
          <span class="state-label">Optimistic:</span>
          <span class="state-val" v-html="activeScenario.alice.visible"></span>
        </div>
      </div>

      <!-- Center OT Bridge -->
      <div class="arena-bridge">
        <div class="bridge-line" aria-hidden="true"></div>
        <div class="bridge-pill">
          <svg class="bridge-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
            <path d="M16 3h5v5M4 20L21 3M21 16v5h-5M15 15l6 6M4 4l5 5"/>
          </svg>
          <span class="bridge-fn">{{ activeScenario.transform.fn }}</span>
          <span class="bridge-time">{{ activeScenario.transform.time }}</span>
        </div>
        <span class="bridge-subtext">{{ activeScenario.transform.metric }}</span>
        <div class="bridge-line" aria-hidden="true"></div>
      </div>

      <!-- Peer B (Bob) -->
      <div class="arena-peer peer-bob">
        <div class="peer-header">
          <div class="peer-identity">
            <span class="peer-dot bob-dot" aria-hidden="true"></span>
            <span class="peer-name">{{ activeScenario.bob.title }}</span>
          </div>
          <span class="peer-rev">{{ activeScenario.bob.rev }}</span>
        </div>
        <div class="peer-code-box">
          <code>{{ activeScenario.bob.code }}</code>
        </div>
        <div class="peer-state-box">
          <span class="state-label">Optimistic:</span>
          <span class="state-val" v-html="activeScenario.bob.visible"></span>
        </div>
      </div>
    </div>

    <!-- Converged Canonical State Result -->
    <div class="workbench-result">
      <div class="result-left">
        <span class="result-badge">
          <svg class="check-icon" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
            <path fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd"/>
          </svg>
          Canonical Converged State
        </span>
        <span class="result-rev">{{ activeScenario.result.rev }}</span>
      </div>
      <div class="result-center">
        <div class="result-preview" v-html="activeScenario.result.text"></div>
        <div class="result-meta">{{ activeScenario.result.note }}</div>
      </div>
      <div class="result-right">
        <span class="result-hash">{{ activeScenario.result.hash }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ot-workbench {
  margin: 36px 0 20px;
  background: var(--vp-c-bg-elv);
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  overflow: hidden;
  box-shadow:
    0 4px 20px -2px rgba(0, 0, 0, 0.05),
    0 1px 3px 0 rgba(0, 0, 0, 0.03),
    inset 0 1px 0 0 rgba(255, 255, 255, 0.06);
  transition: border-color 0.25s ease, box-shadow 0.25s ease;
}

.ot-workbench:hover {
  border-color: rgba(99, 102, 241, 0.35);
  box-shadow:
    0 12px 32px -4px rgba(0, 0, 0, 0.08),
    0 0 0 1px rgba(99, 102, 241, 0.15),
    inset 0 1px 0 0 rgba(255, 255, 255, 0.08);
}

/* Chrome Header */
.workbench-chrome {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: var(--vp-c-bg-soft);
  border-bottom: 1px solid var(--vp-c-divider);
  gap: 12px;
}

.chrome-controls {
  display: flex;
  gap: 6px;
  align-items: center;
}

.control-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  opacity: 0.75;
}

.dot-red { background: #ef4444; }
.dot-yellow { background: #f59e0b; }
.dot-green { background: #10b981; }

.chrome-tabs {
  display: flex;
  gap: 4px;
  background: var(--vp-c-bg);
  padding: 3px;
  border-radius: 999px;
  border: 1px solid var(--vp-c-divider);
}

.chrome-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  border: none;
  background: transparent;
  color: var(--vp-c-text-2);
  font-size: 12px;
  font-weight: 500;
  border-radius: 999px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.chrome-tab:hover {
  color: var(--vp-c-text-1);
}

.chrome-tab.active {
  background: var(--vp-c-brand-1);
  color: #ffffff;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.15);
}

.tab-index {
  font-family: var(--vp-font-family-mono);
  font-size: 10px;
  opacity: 0.8;
}

.chrome-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  color: var(--vp-c-text-3);
}

.live-pulse {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #10b981;
  box-shadow: 0 0 8px #10b981;
  animation: pulse 2s infinite ease-in-out;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.85); }
}

/* Subbar */
.workbench-subbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  background: var(--vp-c-bg-mute);
  border-bottom: 1px solid var(--vp-c-divider);
  font-size: 11px;
}

.subbar-tag {
  font-family: var(--vp-font-family-mono);
  font-weight: 600;
  letter-spacing: 0.06em;
  color: var(--vp-c-brand-1);
}

.subbar-label {
  color: var(--vp-c-text-2);
  font-weight: 500;
}

/* Arena */
.workbench-arena {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  padding: 24px 20px;
  gap: 16px;
}

.arena-peer {
  background: var(--vp-c-bg);
  border: 1px solid var(--vp-c-divider);
  border-radius: 12px;
  padding: 14px;
  transition: transform 0.2s ease, border-color 0.2s ease;
}

.arena-peer:hover {
  transform: translateY(-2px);
}

.peer-alice:hover {
  border-color: rgba(14, 165, 233, 0.4);
}

.peer-bob:hover {
  border-color: rgba(168, 85, 247, 0.4);
}

.peer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.peer-identity {
  display: flex;
  align-items: center;
  gap: 6px;
}

.peer-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.alice-dot {
  background: #0ea5e9;
  box-shadow: 0 0 6px rgba(14, 165, 233, 0.6);
}

.bob-dot {
  background: #a855f7;
  box-shadow: 0 0 6px rgba(168, 85, 247, 0.6);
}

.peer-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--vp-c-text-1);
}

.peer-rev {
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  color: var(--vp-c-text-3);
}

.peer-code-box {
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  padding: 8px 10px;
  margin-bottom: 10px;
}

.peer-code-box code {
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  color: var(--vp-c-text-1);
  word-break: break-all;
}

.peer-state-box {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.state-label {
  color: var(--vp-c-text-3);
  font-size: 11px;
}

.state-val {
  font-family: var(--vp-font-family-mono);
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 6px;
}

.peer-alice .state-val {
  background: rgba(14, 165, 233, 0.1);
  color: #0ea5e9;
  border: 1px solid rgba(14, 165, 233, 0.25);
}

.peer-bob .state-val {
  background: rgba(168, 85, 247, 0.1);
  color: #a855f7;
  border: 1px solid rgba(168, 85, 247, 0.25);
}

/* Center Bridge */
.arena-bridge {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  min-width: 170px;
  text-align: center;
}

.bridge-line {
  width: 1px;
  height: 12px;
  background: var(--vp-c-divider);
}

.bridge-pill {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--vp-c-bg);
  border: 1px solid var(--vp-c-brand-1);
  border-radius: 999px;
  padding: 5px 12px;
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.15);
}

.bridge-icon {
  width: 14px;
  height: 14px;
  color: var(--vp-c-brand-1);
}

.bridge-fn {
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  font-weight: 600;
  color: var(--vp-c-text-1);
}

.bridge-time {
  font-family: var(--vp-font-family-mono);
  font-size: 10px;
  background: rgba(16, 185, 129, 0.15);
  color: #10b981;
  padding: 1px 5px;
  border-radius: 999px;
  font-weight: 600;
}

.bridge-subtext {
  font-family: var(--vp-font-family-mono);
  font-size: 10px;
  color: var(--vp-c-text-3);
  letter-spacing: 0.03em;
}

/* Result Bar */
.workbench-result {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  background: var(--vp-c-bg-soft);
  border-top: 1px solid var(--vp-c-divider);
  gap: 16px;
  flex-wrap: wrap;
}

.result-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.result-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 600;
  color: #10b981;
}

.check-icon {
  width: 14px;
  height: 14px;
}

.result-rev {
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  color: var(--vp-c-text-3);
}

.result-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.result-preview {
  font-family: var(--vp-font-family-mono);
  font-size: 13px;
  font-weight: 600;
  color: var(--vp-c-text-1);
  background: var(--vp-c-bg);
  padding: 4px 14px;
  border-radius: 6px;
  border: 1px solid var(--vp-c-divider);
}

.result-meta {
  font-size: 11px;
  color: var(--vp-c-text-3);
}

.result-right {
  display: flex;
  align-items: center;
}

.result-hash {
  font-family: var(--vp-font-family-mono);
  font-size: 10px;
  color: var(--vp-c-text-3);
  background: var(--vp-c-bg-mute);
  padding: 2px 6px;
  border-radius: 4px;
  border: 1px solid var(--vp-c-divider);
}

/* Responsive */
@media (max-width: 768px) {
  .workbench-arena {
    grid-template-columns: 1fr;
    gap: 12px;
  }

  .arena-bridge {
    flex-direction: row;
    justify-content: center;
    min-width: 0;
  }

  .bridge-line {
    width: 20px;
    height: 1px;
  }

  .workbench-chrome {
    flex-direction: column;
    align-items: flex-start;
  }

  .workbench-result {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
