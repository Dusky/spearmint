import { formatCoordinates } from './format';
import { el, setText } from './dom';
import type { Component } from './component';
import type { GameState } from '../state/types';

/** Modifier-key hints on the left, cursor position in world cells on the right. */
export function createStatusBar(): Component {
  const coordinates = el('span');

  const root = el('div', { class: 'status-bar' }, [
    el('span', {}, ['shift — straight line']),
    el('span', {}, ['space — pan']),
    el('span', {}, ['alt — pick material']),
    el('span', {}, ['t — tooltips']),
    el('div', { class: 'status-bar__spacer' }),
    coordinates,
  ]);

  return {
    root,
    update(state: GameState) {
      setText(coordinates, formatCoordinates(state.ui.cursor.x, state.ui.cursor.y));
    },
  };
}
