import { formatFootprint } from './format';
import { el, setText } from './dom';
import { TILE_CELLS } from '../constants';
import type { Component } from './component';
import type { GameState } from '../state/types';

/**
 * The selection box over the canvas. Its label tab sits above the box and carries the
 * player-given name, then the footprint in tiles.
 *
 * Position is derived from world bounds and the camera, so the marquee tracks the
 * selection while panning rather than being placed in screen space.
 */
export function createMarquee(viewport: HTMLElement): Component {
  const label = el('div', { class: 'marquee__label' });
  const root = el('div', { class: 'marquee' }, [label]);

  return {
    root,

    update(state: GameState) {
      const construct = state.constructs.find((item) => item.id === state.ui.selection);
      root.hidden = !construct;
      if (!construct) return;

      const { camera } = state.ui;
      const { bounds } = construct;
      const width = viewport.clientWidth;
      const height = viewport.clientHeight;

      // World cells -> screen pixels, about the viewport centre.
      const left = (bounds.x * TILE_CELLS - camera.x) * camera.zoom + width / 2;
      const top = (bounds.y * TILE_CELLS - camera.y) * camera.zoom + height / 2;

      root.style.left = `${Math.round(left)}px`;
      root.style.top = `${Math.round(top)}px`;
      root.style.width = `${Math.round(bounds.width * TILE_CELLS * camera.zoom)}px`;
      root.style.height = `${Math.round(bounds.height * TILE_CELLS * camera.zoom)}px`;

      setText(label, `${construct.name} · ${formatFootprint(bounds.width, bounds.height)} tiles`);
    },
  };
}
