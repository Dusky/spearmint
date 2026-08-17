import { formatFootprint, formatPercent } from '../format';
import { renderBlueprintThumbnail } from '../blueprintThumbnail';
import { el, setText } from '../dom';
import type { Component } from '../component';
import type { Blueprint, GameState } from '../../state/types';

/** "24×20 · 82%" — footprint, then best yield observed. Footprint only for a
 *  construct that has no output. */
function metaFor(blueprint: Blueprint): string {
  const footprint = formatFootprint(blueprint.width, blueprint.height);
  return blueprint.yieldBest === null
    ? footprint
    : `${footprint} · ${formatPercent(blueprint.yieldBest)}%`;
}

function card(blueprint: Blueprint): HTMLElement {
  return el('div', { class: 'blueprint-card' }, [
    el('div', { class: 'blueprint-card__thumb' }, [renderBlueprintThumbnail(blueprint)]),
    el('div', { class: 'blueprint-card__name' }, [blueprint.name]),
    el('div', { class: 'blueprint-card__meta' }, [metaFor(blueprint)]),
  ]);
}

export function createBlueprintsDrawer(): Component {
  const count = el('span', { class: 'drawer__count' });
  const grid = el('div', { class: 'blueprints__grid' });

  const root = el('div', { class: 'drawer blueprints' }, [
    el('div', { class: 'drawer__title-row' }, [
      el('span', { class: 'drawer__title' }, ['Blueprints']),
      count,
    ]),
    el('div', { class: 'drawer__subtitle' }, ['Thumbnails are the real thing, not an icon.']),
    el('div', { class: 'drawer__rule' }),
    grid,
  ]);

  // Thumbnails are rasterised from cell data, so they are rebuilt only when the
  // blueprint set itself changes.
  let rendered: readonly Blueprint[] | null = null;
  let renderedSlots = -1;

  return {
    root,

    update(state: GameState) {
      root.hidden = state.ui.openDrawer !== 'blueprints';
      if (root.hidden) return;

      const slots = state.economy.blueprintSlots;
      setText(count, `${state.blueprints.length} / ${slots}`);

      if (rendered === state.blueprints && renderedSlots === slots) return;
      rendered = state.blueprints;
      renderedSlots = slots;

      grid.replaceChildren(
        ...state.blueprints.map(card),
        ...Array.from({ length: Math.max(0, slots - state.blueprints.length) }, () =>
          el('div', { class: 'blueprint-slot--empty' }, ['empty']),
        ),
      );
    },
  };
}
