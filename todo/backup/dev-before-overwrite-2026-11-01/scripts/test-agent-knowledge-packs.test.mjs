import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const manifest = JSON.parse(readFileSync(new URL('./fixtures/technical-analysis-master-list.v1.json', import.meta.url), 'utf8'));
const agentFiles = [
  ['SMC & ICT', '../docs/agents/01-smc-ict-agent.md'],
  ['Price Action', '../docs/agents/02-price-action-agent.md'],
  ['Support, Resistance & Trend', '../docs/agents/03-support-resistance-trend-agent.md'],
  ['Indikator Teknikal', '../docs/agents/04-technical-indicators-agent.md'],
  ['Order Flow & Market Profile', '../docs/agents/05-order-flow-market-profile-agent.md'],
  ['Divergence & Intermarket Analysis', '../docs/agents/06-divergence-intermarket-agent.md'],
  ['Metode Analisis Lain', '../docs/agents/07-other-methods-agent.md'],
  ['Fundamental & Makro', '../docs/agents/08-fundamental-macro-agent.md'],
  ['Risk Management & Evaluation', '../docs/agents/09-risk-management-evaluation-agent.md'],
];

test('nine agent knowledge packs cover every authoritative catalog ID exactly once', () => {
  const assigned = new Map();
  for (const [category, relativePath] of agentFiles) {
    const body = readFileSync(new URL(relativePath, import.meta.url), 'utf8');
    assert.ok(body.includes(`## Complete category inventory`), `${category} inventory heading missing`);
    const entries = manifest.entries.filter((entry) => entry.category === category);
    assert.ok(entries.length > 0, `No entries mapped to ${category}`);
    for (const entry of entries) {
      assert.ok(body.includes(`\`${entry.id}\``), `Knowledge pack missing ${entry.id}`);
      assigned.set(entry.id, (assigned.get(entry.id) ?? 0) + 1);
    }
  }
  assert.equal(assigned.size, 364);
  assert.ok([...assigned.values()].every((count) => count === 1));
});

test('all specialist packs and orchestrator state the fail-closed and no-live-execution contract', () => {
  for (const [, relativePath] of agentFiles) {
    const body = readFileSync(new URL(relativePath, import.meta.url), 'utf8');
    assert.match(body, /Never bypass the central strict decision gate/);
    assert.match(body, /Live execution is not authorized/);
  }
  const orchestrator = readFileSync(new URL('../docs/agents/00-decision-orchestrator.md', import.meta.url), 'utf8');
  assert.match(orchestrator, /full 364-entry inventory/);
  assert.match(orchestrator, /unsupported/);
});
