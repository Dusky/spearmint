/* Rules shared by building and by the preview of building.
 *
 * The ghost has to agree with what a click will actually do — if the two drift, the
 * preview becomes a lie, which is worse than no preview. So the rules live here once
 * and both sides call them.
 */

import { TILE_CELLS } from '../constants';
import { canBeEmitted, elementByName } from '../sim/elements';
import { entityForTool } from '../sim/entities';
import type { GameState, Vec2 } from './types';

/** World cell to tile. Everything the player builds is on the tile grid (spec 2.3). */
export const toTile = (cell: number): number => Math.floor(cell / TILE_CELLS);

/** Both ends of a stroke in tile coordinates. */
export const strokeTiles = (from: Vec2, to: Vec2): { start: Vec2; end: Vec2 } => ({
  start: { x: toTile(from.x), y: toTile(from.y) },
  end: { x: toTile(to.x), y: toTile(to.y) },
});

/** Holding shift constrains a stroke to whichever axis it has travelled furthest along. */
export function constrainToAxis(start: Vec2, end: Vec2): Vec2 {
  return Math.abs(end.x - start.x) >= Math.abs(end.y - start.y)
    ? { x: end.x, y: start.y }
    : { x: start.x, y: end.y };
}

/**
 * Why placing the selected machine here would be refused, or null if it would work.
 *
 * The reason is a phrase rather than a boolean because the ghost showing *that* it is
 * refused is half the job; saying why is the other half, whenever there is somewhere to
 * say it.
 */
export function placementRefusal(state: GameState): string | null {
  const { selectedTool, selectedMaterial } = state.ui;
  if (!entityForTool(selectedTool)) return null;

  // Only emitters have refusals so far: a press and a vault are uncapped, because the
  // cap that matters is on input (spec 3.4).
  if (selectedTool !== 'spawner') return null;

  const { spawnersOwned, spawnersMax } = state.economy;
  if (spawnersOwned >= spawnersMax) return 'every spawner slot is in use';
  if (!canBeEmitted(elementByName(selectedMaterial))) {
    return `${selectedMaterial} does not flow — an emitter has nothing to emit`;
  }
  return null;
}
