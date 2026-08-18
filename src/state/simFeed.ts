import { elementByName } from '../sim/elements';
import { entityForTool } from '../sim/entities';
import { RATE_WINDOW_MS, READOUT_HZ } from '../constants';
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
  const gold = elementByName('gold').id;
  const emitterKind = entityForTool('spawner')?.id ?? 0;
  const collectorKind = entityForTool('collector')?.id ?? 0;

  let last = performance.now();
  let tickDebt = 0;
  let sinceReadout = 0;
  /** Recent revenue samples, for a rate measured over a window rather than over one
   *  readout. A single 250ms sample reads zero often enough in a working factory to
   *  make both the rate and the panel's diagnosis flicker. */
  const revenue: { at: number; collected: number }[] = [];
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

    const sandCells = sim.count(sand);
    const wetCells = sim.count(wetSand);
    const washable = sandCells + wetCells;

    // The balance is a pile of nuggets in a machine, measured like any other physical
    // quantity. Income is separate: nuggets ever minted, which is what a rate wants —
    // a balance falls when you spend, and that is not the same question.
    const collected = sim.collected;
    const stored = sim.stored;
    revenue.push({ at: now, collected });
    while (revenue.length > 1 && now - (revenue[0]?.at ?? now) > RATE_WINDOW_MS) revenue.shift();
    const oldest = revenue[0] ?? { at: now, collected };
    const span = (now - oldest.at) / 1000;
    const goldRate = span > 0 ? (collected - oldest.collected) / span : 0;

    store.update((state) => ({
      ...state,
      tick: sim.tick,
      economy: {
        ...state.economy,
        gold: stored,
        goldRate,
        spawnersOwned: sim.countOfKind(emitterKind),
        collectorsOwned: sim.countOfKind(collectorKind),
      },
      readout: {
        yieldCurrent: washable > 0 ? wetCells / washable : 0,
        looseGold: sim.count(gold) - stored,
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
