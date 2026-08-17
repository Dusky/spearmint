/* Inspector measurements and the diagnosis sentence.
 *
 * The panel's job is to explain *why* a machine underperforms: it reports physical
 * conditions and names the bottleneck in a sentence. It never reports an abstracted
 * throughput number (spec §1.1, §3.3, §6).
 *
 * CONTENT PLACEHOLDER — the ranges below and every sentence except the residence-low
 * one (which is the designed copy) are stand-ins. Real thresholds and phrasing depend
 * on the element roster and tech tree, which are still open (spec §11 q7). The
 * *mechanism* — pick whichever measurement is furthest out of range, phrase it — is
 * what this file settles. */

import { formatDecimal, formatInteger } from '../ui/format';
import type { SimReadout } from './types';

export type MeasurementKey = 'temperature' | 'contactArea' | 'residence' | 'mixing';

export interface MeasurementSpec {
  readonly key: MeasurementKey;
  readonly label: string;
  /** Inclusive band the value should sit within. */
  readonly ok: readonly [number, number];
  readonly format: (value: number) => string;
  /** Diagnosis when this measurement is the furthest below its band. */
  readonly whenLow: string;
  /** Diagnosis when it is the furthest above. */
  readonly whenHigh: string;
  /** Read out when nothing is out of range and this sits closest to its band edge. */
  readonly whenLimiting: string;
}

export const MEASUREMENTS: readonly MeasurementSpec[] = [
  {
    key: 'temperature',
    label: 'temperature',
    ok: [340, 420],
    format: (value) => `${formatInteger(value)} K`,
    whenLow: 'Temperature is low — the reaction barely proceeds. Add heaters or insulate the chamber.',
    whenHigh: 'Temperature is high — water boils off before it reaches the sand. Back the heaters off or vent the chamber.',
    whenLimiting: 'Nothing is out of range. Temperature sits closest to its limit and sets the ceiling here.',
  },
  {
    key: 'contactArea',
    label: 'contact area',
    ok: [1200, 4000],
    format: (value) => `${formatInteger(value)} cells`,
    whenLow: 'Contact area is small — most of the sand never meets water. Widen the bed or spread the inflow.',
    whenHigh: 'Contact area outruns the feed — the bed is larger than the inflow can wet. Narrow it or feed it harder.',
    whenLimiting: 'Nothing is out of range. Contact area sits closest to its limit and sets the ceiling here.',
  },
  {
    key: 'residence',
    label: 'residence',
    ok: [60, 140],
    format: (value) => `${formatInteger(value)} ticks`,
    whenLow: 'Residence is short — water leaves before it wets the sand. Narrow the outlet or raise the weir.',
    whenHigh: 'Residence is long — material sits after it has finished reacting. Widen the outlet or lower the weir.',
    whenLimiting: 'Nothing is out of range. Residence sits closest to its limit and sets the ceiling here.',
  },
  {
    key: 'mixing',
    label: 'mixing',
    ok: [0.4, 0.9],
    format: (value) => formatDecimal(value, 2),
    whenLow: 'Mixing is poor — the two streams pass without meeting. Add a baffle or stagger the inflow.',
    whenHigh: 'Mixing is aggressive — the bed re-suspends before it can settle. Slow the inflow.',
    whenLimiting: 'Nothing is out of range. Mixing sits closest to its limit and sets the ceiling here.',
  },
];

export type Deviation = 'low' | 'high' | null;

/** Which side of its band a value falls on, or null when it is inside. */
export function deviationOf(spec: MeasurementSpec, value: number): Deviation {
  const [min, max] = spec.ok;
  if (value < min) return 'low';
  if (value > max) return 'high';
  return null;
}

/**
 * How far outside the band a value sits, as a fraction of the band's width — so
 * kelvin, cells, ticks and a bare ratio stay comparable. Zero when inside.
 */
function excess(spec: MeasurementSpec, value: number): number {
  const [min, max] = spec.ok;
  const span = max - min;
  if (value < min) return (min - value) / span;
  if (value > max) return (value - max) / span;
  return 0;
}

/** How much room is left before the nearer edge, as a fraction of the band's width. */
function margin(spec: MeasurementSpec, value: number): number {
  const [min, max] = spec.ok;
  const span = max - min;
  return Math.min(value - min, max - value) / span;
}

/**
 * The diagnosis sentence: derived from whichever measurement is furthest out of range.
 * When everything is in range it reads as a plain statement of what limits the machine.
 */
export function diagnose(readout: SimReadout): string {
  let worst: MeasurementSpec | null = null;
  let worstExcess = 0;

  for (const spec of MEASUREMENTS) {
    const over = excess(spec, readout[spec.key]);
    if (over > worstExcess) {
      worstExcess = over;
      worst = spec;
    }
  }

  if (worst) {
    return deviationOf(worst, readout[worst.key]) === 'low' ? worst.whenLow : worst.whenHigh;
  }

  let nearest = MEASUREMENTS[0];
  if (!nearest) return '';
  let nearestMargin = margin(nearest, readout[nearest.key]);
  for (const spec of MEASUREMENTS) {
    const room = margin(spec, readout[spec.key]);
    if (room < nearestMargin) {
      nearestMargin = room;
      nearest = spec;
    }
  }
  return nearest.whenLimiting;
}
