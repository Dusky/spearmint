import type { GameState } from '../state/types';

/** Every HUD component builds its DOM once and writes into it on update. Nothing is
 *  re-created per frame, and there are no transitions anywhere in the design. */
export interface Component {
  readonly root: HTMLElement;
  update(state: GameState): void;
}
