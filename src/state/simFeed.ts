import { elementByName } from '../sim/elements';
import { entityForTool } from '../sim/entities';
import { READOUT_HZ } from '../constants';
import type { GameState } from './types';
import type { Sim } from '../sim/wasm';
import type { Store } from './store';

/**
 * Drives the simulation and reports it into the store.
 *
 * The fixed timestep lives here rather than in the sim (spec 3.1): the core exposes a
 * pure `step`, and the host decides how ticks relate to wall-clock time.
 *
 * Readouts are written at READOUT_HZ, not per frame. Partly because per-frame numbers
 * are unreadable, and partly because measuring the world means walking it — contact
 * area in particular is a full scan, and doing that sixty times a second to produce a
 * number that changes too fast to read would be pure waste.
 */
export function startSimFeed(store: Store<GameState>, sim: Sim): () => void {
  const tickMs = 1000 / store.state.tickRate;
  const readoutMs = 1000 / READOUT_HZ;

  const sand = elementByName('sand').id;
  const water = elementByName('water').id;
  const wetSand = elementByName('wetSand').id;
  const emitterKind = entityForTool('spawner')?.id ?? 0;

  let last = performance.now();
  let tickDebt = 0;
  let sinceReadout = 0;
  let previousCollected = 0;
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
    const seconds = sinceReadout / 1000;
    sinceReadout = 0;

    const sandCells = sim.count(sand);
    const wetCells = sim.count(wetSand);
    const washable = sandCells + wetCells;

    // Revenue comes from the sim — only a collector makes gold, and only by taking
    // product out of the world. Spending is the client's ledger until there is a
    // server (spec 8.1), so the balance is the difference.
    const collected = sim.collected;
    const goldRate = (collected - previousCollected) / seconds;
    previousCollected = collected;

    store.update((state) => ({
      ...state,
      tick: sim.tick,
      economy: {
        ...state.economy,
        gold: collected - state.economy.spent,
        goldRate,
        spawnersOwned: sim.countOfKind(emitterKind),
      },
      readout: {
        yieldCurrent: washable > 0 ? wetCells / washable : 0,
        contactArea: sim.contactArea,
        sand: sandCells,
        water: sim.count(water),
        wetSand: wetCells,
      },
    }));
  };

  frame = requestAnimationFrame(loop);
  return () => cancelAnimationFrame(frame);
}
