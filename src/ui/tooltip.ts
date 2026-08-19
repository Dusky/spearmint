import { TILE_CELLS } from '../constants';
import { el, setText } from './dom';
import type { Component } from './component';
import type { GameState } from '../state/types';

/**
 * Names whatever is under the cursor.
 *
 * The roster has outgrown the point where a silhouette says what something is: eight
 * machines and ten elements, several of which are variations on the same yellow. What
 * it says is computed in `actions.describeAt` as the cursor moves; this only places it.
 *
 * Positioned in world space against the camera, like the ghost and the marquee, so it
 * sits on the thing it describes rather than trailing the pointer in screen space.
 */
export function createTooltip(viewport: HTMLElement): Component {
  const label = el('span', { class: 'tooltip__label' });
  const root = el('div', { class: 'tooltip' }, [label]);

  return {
    root,

    update(state: GameState) {
      const { hoverLabel, cursor, camera, showTooltips } = state.ui;
      root.hidden = !showTooltips || !hoverLabel;
      if (!showTooltips || !hoverLabel) return;

      setText(label, hoverLabel);

      // Offset by a tile so the label sits beside the cursor rather than under it,
      // where the thing being described is.
      const left = (cursor.x - camera.x) * camera.zoom + viewport.clientWidth / 2 + TILE_CELLS;
      const top = (cursor.y - camera.y) * camera.zoom + viewport.clientHeight / 2 + TILE_CELLS;
      root.style.left = `${Math.round(left)}px`;
      root.style.top = `${Math.round(top)}px`;
    },
  };
}
