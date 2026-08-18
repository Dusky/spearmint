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

/** Spawner slots the player starts with. The cap on spawners is the game's only hard
 *  limit on production (spec 3.4). */
export const BASE_SPAWNERS = 5;

/** What the next spawner slot costs.
 *
 *  Doubling, because the cap is the one real limit on production: widening it should
 *  always be the largest thing gold can do, and should always be getting harder. */
export function spawnerSlotPrice(spawnersMax: number): number {
  return 100 * 2 ** (spawnersMax - BASE_SPAWNERS);
}
