'use strict';
const $ = id => document.getElementById(id);
const brain = new BrainView($('brain'));
const shogiBoard = window.shogiBoard;
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let state = null, selected = null, busy = false, revision = 0;
let pendingPromotion = [];

function render() {
  if (state) {
    shogiBoard.render(state, selected, busy || state.turn !== 'b' || Boolean(state.terminal));
    $('stage').dataset.ply = state.moves.length;
    $('stage').dataset.turn = state.turn;
  }
}
function showError(error) { $('status').textContent = error.message || String(error); }
async function api(action, moves = state?.moves || [], extra = {}) {
  const response = await fetch('/api', {method:'POST', headers:{'Content-Type':'application/json'},
    body:JSON.stringify({action, moves, seed:101, ...extra})});
  const data = await response.json();
  if (!response.ok || data.error) throw Error(data.error || 'Unable to connect.');
  if (data.notice) throw Error(data.notice);
  return data;
}
function reward(value) {
  const amount = value?.external_reward ?? 0;
  $('reward-value').textContent = value ? `${(amount * 100).toFixed(2)}%` : '—';
  $('reward').classList.toggle('received', Boolean(value));
  $('reward').dataset.amount = value ? String(amount) : '';
  const meter = $('reward');
  meter.setAttribute('aria-valuenow', String(amount * 100));
  if (value) brain.reward(amount);
}
async function reset() {
  const token = ++revision;
  busy = true; selected = null; $('promotion').close(); brain.clear(); reward(null);
  $('status').textContent = ''; render();
  try {
    const data = await api('state', []);
    if (token === revision) state = data.state;
  } catch (error) { if (token === revision) showError(error); }
  finally { if (token === revision) { busy = false; render(); } }
}
async function humanMove(move) {
  if (busy || !state || state.turn !== 'b' || state.terminal) return;
  const token = revision;
  busy = true; selected = null; $('status').textContent = ''; render();
  try {
    const human = await api('play', state.moves, {move_usi:move});
    if (token !== revision) return;
    state = human.state; render();
    if (state.terminal) { $('status').textContent = state.terminal; return; }
    const fly = await api('think');
    if (token !== revision) return;
    if (!await brain.ready) throw Error('Unable to load brain coordinates.');
    if (token !== revision) return;
    brain.play(fly.telemetry, 2000);
    await sleep(2000);
    if (token !== revision) return;
    state = fly.state; render();
    await sleep(350);
    if (token !== revision) return;
    reward(fly.feedback);
    await sleep(3200);
    if (token === revision && state.terminal) $('status').textContent = state.terminal;
  } catch (error) { if (token === revision) showError(error); }
  finally { if (token === revision) { busy = false; render(); } }
}
function clickSquare(square) {
  if (!state || busy || state.turn !== 'b' || state.terminal) return;
  const options = selected ? state.legal.filter(m => m.startsWith(selected) && m.slice(2,4) === square) : [];
  if (options.length === 1) { void humanMove(options[0]); return; }
  if (options.length > 1) { pendingPromotion = options; $('promotion').showModal(); return; }
  const piece = state.cells.find(c => c.square === square && c.side === 'b');
  selected = piece && selected !== square ? square : null; render();
}
shogiBoard.setHandlers(clickSquare, (piece, color) => {
  if (!state || busy || state.turn !== 'b' || state.terminal || color !== 'black') return;
  selected = selected === piece+'*' ? null : piece+'*'; render();
});
for (const [id, promote] of [['promote-yes',true],['promote-no',false]]) {
  $(id).onclick = () => { $('promotion').close(); void humanMove(pendingPromotion.find(m => m.endsWith('+') === promote)); };
}
$('promote-cancel').onclick = () => $('promotion').close();
$('reset').onclick = reset;
void reset();
