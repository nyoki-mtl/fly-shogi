const test = require('node:test');
const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const vm = require('node:vm');

test('activity started within a frame never selects a negative bin', () => {
  const label = { textContent: '' };
  const scope = { document: { getElementById: () => label, querySelectorAll: () => [], body: { classList: { remove() {} } } } };
  vm.createContext(scope);
  vm.runInContext(readFileSync('web/brain.js', 'utf8') + '\nthis.BrainView = BrainView;', scope);
  const noop = () => {};
  const ctx = new Proxy({createLinearGradient:()=>({addColorStop:noop})}, { get: (target, key) => target[key] ?? noop });
  const view = Object.assign(Object.create(scope.BrainView.prototype), {
    ctx, width: 640, height: 400, nodes: [{ p: [0,0,0], group: 'kc' }],
    data: { brain_activity: [[2],[3],[4],[5]], simulation_ms: 40, observation_start_ms: 0 },
    started: 100, duration: 2000, max: 5, angle: 0, reduced: true,
    rewardAt: -Infinity, sprite: { kc: {} }
  });
  assert.doesNotThrow(() => view.draw(99));
  assert.equal(view.currentMs, 0);
  assert.doesNotThrow(() => view.draw(5000));
  assert.equal(view.currentMs, 40);
});

test('external nectar arrives before the conceptual brain feedback wave', () => {
  const scope = {};
  vm.createContext(scope);
  vm.runInContext(readFileSync('web/brain.js', 'utf8') + '\nthis.BrainView = BrainView;', scope);
  const view = Object.assign(Object.create(scope.BrainView.prototype), {rewardAt:100, rewardAmount:.34});
  const filling = view.feedbackPhase(450);
  assert.equal(filling.fill, .17);
  assert.equal(filling.travel, 0);
  assert.ok(filling.wave < 0);
  const travelling = view.feedbackPhase(1100);
  assert.equal(travelling.fill, .34);
  assert.ok(travelling.travel > 0 && travelling.travel < 1);
  assert.ok(travelling.wave < 0);
  assert.ok(view.feedbackPhase(1600).wave > 0);
  assert.equal(view.feedbackPhase(5000).fill, .34);
});
