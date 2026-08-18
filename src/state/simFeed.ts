import { READOUT_HZ } from '../constants';
import type { GameState } from './types';
import type { Sim } from '../sim/wasm';
import type { Store } from './store';

/**
 * Drives the simulation and reports it into the store.
 *
 * The fixed timestep lives here rather than in the sim (spec 3.1): the core exposes a
 * pure `step`, and the host decides how ticks relate to wall-clock time. Readouts are
 * written at READOUT_HZ, not per frame — per-frame is unreadable and wasteful, and
 * monospace numerals keep them from reflowing between writes.
 */
export function startSimFeed(store: Store<GameState>, sim: Sim): () => void {
  const tickMs = 1000 / store.state.tickRate;
  const readoutMs = 1000 / READOUT_HZ;

  let last = performance.now();
  let tickDebt = 0;
  let sinceReadout = 0;
  let frame = 0;

  const loop = (now: number): void => {
    frame = requestAnimationFrame(loop);

    // A backgrounded tab must not come back and simulate a minute in one frame.
    const elapsed = Math.min(now - last, 250);
    last = now;

    tickDebt += elapsed;
    const ticks = Math.floor(tickDebt / tickMs);
    tickDebt -= ticks * tickMs;
    if (ticks > 0) sim.step(ticks);

    sinceReadout += elapsed;
    if (sinceReadout < readoutMs) return;
    sinceReadout = 0;

    store.update((state) => ({ ...state, tick: sim.tick }));
  };

  frame = requestAnimationFrame(loop);
  return () => cancelAnimationFrame(frame);
}
