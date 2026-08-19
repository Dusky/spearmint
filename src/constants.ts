/** One tile is 9x9 particle cells (spec §2.3). Odd on purpose: every tile has a true
 *  centre cell, which matters for rotation, symmetry, and teleporter endpoints.
 *  Final tile size is still open (spec §11 q2), so nothing hard-codes 9. */
export const TILE_CELLS = 9;

/** Rendered pixels per particle cell. The handoff draws the tile grid at 36px, which
 *  is one tile at 4x. */
export const DEFAULT_ZOOM = 4;

/** Readouts are refreshed at this rate, not per frame. Per-frame is unreadable and
 *  wasteful; ~4 Hz is enough. Monospace keeps the values from reflowing between
 *  updates. */
export const READOUT_HZ = 4;

/** How far back the gold rate looks. Long enough that a working factory never reads
 *  zero between mouthfuls — the panel treats a zero rate as "product is not reaching a
 *  press", and that has to mean it. */
export const RATE_WINDOW_MS = 2000;

/** Room temperature, in Kelvin, and what an untouched cell reads. Mirrors
 *  `heat::AMBIENT_TEMPERATURE` in the core, which is the authority — this copy exists
 *  so the panel can tell "nobody has heated anything" apart from "heat is being made
 *  and is not arriving", and is only ever compared against, never simulated with. */
export const AMBIENT_TEMPERATURE = 293;

/** Speeds the sim can be watched at, as multiples of `tickRate`. Zero is paused.
 *
 *  Halves and doubles rather than a slider: the reason to change speed is to watch
 *  something specific, and a handful of steps you can hit with one click beats a
 *  continuous control you have to aim. */
export const SPEEDS = [0, 0.25, 0.5, 1, 2] as const;

/** Spawner slots the player starts with. The cap on spawners is the game's only hard
 *  limit on production (spec 3.4). */
export const BASE_SPAWNERS = 5;

/** What the next spawner slot costs, in nuggets.
 *
 *  Doubling, because the cap is the one real limit on production: widening it should
 *  always be the largest thing gold can do, and should always be getting harder. The
 *  numbers are small because a nugget is a cell: the third slot costs more gold than a
 *  one-tile vault can hold, which is the point. */
export function spawnerSlotPrice(spawnersMax: number): number {
  return 25 * 2 ** (spawnersMax - BASE_SPAWNERS);
}
