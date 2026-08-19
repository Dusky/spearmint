import { constrainToAxis, placementRefusal, toTile } from '../state/build';
import { el, setClass } from './dom';
import { entityForTool, isToolBuilt } from '../sim/entities';
import { TILE_CELLS } from '../constants';
import type { Component } from './component';
import type { GameState, TileRect } from '../state/types';

/**
 * What the next click will do, drawn before it does it.
 *
 * Everything the player builds is tile-aligned (spec 2.3), so the ghost is always a
 * whole number of tiles: one for a stroke, a machine's footprint for a machine, and the
 * whole line for a shift-constrained stroke that has not committed yet.
 *
 * The rules it draws come from `state/build.ts`, the same module the actions use. A
 * preview that can disagree with what happens is worse than no preview.
 *
 * Positioning follows the marquee: world cells converted against the camera, so it
 * tracks while panning rather than sitting in screen space.
 */
export function createGhost(viewport: HTMLElement): Component {
  const root = el('div', { class: 'ghost' });

  return {
    root,

    update(state: GameState) {
      const rect = ghostRect(state);
      root.hidden = !rect;
      if (!rect) return;

      const { camera } = state.ui;
      const left = (rect.x * TILE_CELLS - camera.x) * camera.zoom + viewport.clientWidth / 2;
      const top = (rect.y * TILE_CELLS - camera.y) * camera.zoom + viewport.clientHeight / 2;

      root.style.left = `${Math.round(left)}px`;
      root.style.top = `${Math.round(top)}px`;
      root.style.width = `${Math.round(rect.width * TILE_CELLS * camera.zoom)}px`;
      root.style.height = `${Math.round(rect.height * TILE_CELLS * camera.zoom)}px`;
      setClass(root, 'ghost--refused', placementRefusal(state) !== null);
    },
  };
}

/** The tiles the current tool would affect, or null when there is nothing to show. */
function ghostRect(state: GameState): TileRect | null {
  const { selectedTool, cursor, strokeAnchor, painting } = state.ui;
  if (!isToolBuilt(selectedTool)) return null;
  // Select places nothing, so previewing a footprint for it would promise a build that
  // is never going to happen.
  if (selectedTool === 'select') return null;
  // While a free stroke is being painted the paint is the feedback; a box chasing the
  // cursor over it is just noise.
  if (painting && !strokeAnchor) return null;

  const at = { x: toTile(cursor.x), y: toTile(cursor.y) };

  if (strokeAnchor) {
    const start = { x: toTile(strokeAnchor.x), y: toTile(strokeAnchor.y) };
    // A vault is marked out as an area; a belt or filter line is horizontal only,
    // whichever way the drag actually wandered (spec 4's open question on vertical
    // transport is unresolved); a constrained stroke picks whichever axis moved most.
    const end =
      selectedTool === 'vault'
        ? at
        : selectedTool === 'belt' || selectedTool === 'filter'
          ? { x: at.x, y: start.y }
          : constrainToAxis(start, at);
    return {
      x: Math.min(start.x, end.x),
      y: Math.min(start.y, end.y),
      width: Math.abs(end.x - start.x) + 1,
      height: Math.abs(end.y - start.y) + 1,
    };
  }

  const machine = entityForTool(selectedTool);
  return {
    x: at.x,
    y: at.y,
    width: machine?.widthTiles ?? 1,
    height: machine?.heightTiles ?? 1,
  };
}
