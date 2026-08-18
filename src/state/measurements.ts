/* What the inspector reports, and why.
 *
 * Spec 3.3: reactions have no fixed conversion ratio, so yield is whatever the layout
 * produces. The interface's job is therefore not to show a throughput number — it is to
 * explain why the machine is doing what it is doing (spec 1.1, 6).
 *
 * Everything here is derived from measured state. Nothing is modelled, and nothing is
 * invented to fill a row in the panel.
 */

import { formatInteger, formatPercent } from '../ui/format';
import type { GameState, SimReadout } from './types';

export interface Measurement {
  readonly label: string;
  readonly value: string;
  /** The one highlight in the panel: this reading is why the machine is limited. */
  readonly limiting: boolean;
}

export function measurementsOf(readout: SimReadout): readonly Measurement[] {
  return [
    {
      label: 'contact area',
      value: `${formatInteger(readout.contactArea)} cells`,
      // Reactants that never touch cannot react, whatever else is right.
      limiting: readout.contactArea === 0 && readout.sand > 0 && readout.water > 0,
    },
    { label: 'sand', value: `${formatInteger(readout.sand)} cells`, limiting: false },
    { label: 'water', value: `${formatInteger(readout.water)} cells`, limiting: false },
    {
      label: 'washed',
      value: `${formatInteger(readout.wetSand)} cells`,
      limiting: false,
    },
    {
      label: 'gold on the floor',
      value: `${formatInteger(readout.looseGold)} nuggets`,
      // Money that is not being held is money you do not have.
      limiting: readout.looseGold > 0,
    },
  ];
}

/**
 * One sentence naming the bottleneck.
 *
 * Ordered by what a player can act on first: nothing feeding it, nothing to react with,
 * or reactants that have stopped touching. The last is the interesting one — product
 * builds up between the reactants and cuts the contact area to nothing, so a machine
 * chokes on what it has made. That emerged from the physics; it was not designed in.
 */
export function diagnose(state: GameState): string {
  const readout = state.readout;
  if (!readout) return '';

  if (state.economy.spawnersOwned === 0) {
    return 'No spawners. Nothing is feeding the machine — place one with the spawner tool.';
  }
  if (readout.sand === 0 && readout.water === 0) {
    return 'The machine is empty. Material from the spawners has not reached it yet.';
  }
  if (readout.sand === 0) {
    return 'No sand. Water alone has nothing to wash.';
  }
  if (readout.water === 0) {
    return 'No water. Sand alone cannot be washed.';
  }
  // Gold before contact: product that exists and is not being sold is the more
  // actionable problem, and the one with no feedback anywhere else on screen.
  if (readout.wetSand > 0 && state.economy.collectorsOwned === 0) {
    return 'Washed sand is piling up with nowhere to go. Gold only moves when product reaches a collector.';
  }
  if (readout.wetSand > 0 && state.economy.goldRate === 0) {
    return 'Product is not reaching a collector. Check that it can fall in, and that the collector has a floor under it — anything that falls through is gone.';
  }
  // Currency is matter, so it can be spilled. A pile on the floor is gold and is not
  // money, and nothing else on screen would say so.
  if (readout.looseGold > 0 && state.economy.gold === 0) {
    return `${formatInteger(readout.looseGold)} nuggets are lying on the floor. Gold only counts while a machine is holding it — walls that keep it inside the collector are what make it money.`;
  }
  if (readout.contactArea === 0) {
    return readout.wetSand > 0
      ? 'Sand and water have stopped touching — washed sand has settled between them. Keep them mixing, or take the product out.'
      : 'Sand and water are not touching. Nothing can react until they meet.';
  }
  if (readout.contactArea < 40) {
    return `Only ${formatInteger(readout.contactArea)} cells of contact. Yield is limited by how much sand meets water, not by how much of either you have.`;
  }
  return `${formatPercent(readout.yieldCurrent)}% of the sand is washed. Widen the contact between sand and water to raise it.`;
}
