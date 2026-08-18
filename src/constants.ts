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
 *  collector", and that has to mean it. */
export const RATE_WINDOW_MS = 2000;

/** Spawner slots the player starts with. The cap on spawners is the game's only hard
 *  limit on production (spec 3.4). */
export const BASE_SPAWNERS = 5;

/** What the next spawner slot costs, in nuggets.
 *
 *  Doubling, because the cap is the one real limit on production: widening it should
 *  always be the largest thing gold can do, and should always be getting harder. The
 *  numbers are small because a nugget is a cell and a collector body holds 81 of them —
 *  the third slot costs more than one machine can hold, which is the point. */
export function spawnerSlotPrice(spawnersMax: number): number {
  return 25 * 2 ** (spawnersMax - BASE_SPAWNERS);
}
