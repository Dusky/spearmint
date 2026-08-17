import { el, setText } from './dom';
import type { Component } from './component';
import type { GameState, NoticeId } from '../state/types';

interface NoticeActions {
  dismiss(id: NoticeId): void;
}

/**
 * Clogging is emergent physics, not a rule (spec §4.2), so this is informational and
 * not an alarm: no red, no modal, no sound. It states the belt, the material and the
 * depth, offers a jump key, and gets out of the way.
 *
 * Only one notice is shown — the most recent. Handling several at once is undesigned
 * (handoff open question 3).
 */
export function createNotice(actions: NoticeActions): Component {
  const message = el('span', { class: 'notice__message' });
  const root = el('div', { class: 'notice' }, [
    message,
    el('span', { class: 'notice__hint' }, ['F — jump there']),
  ]);

  let shown: NoticeId | null = null;
  root.addEventListener('pointerdown', () => {
    if (shown) actions.dismiss(shown);
  });

  return {
    root,

    update(state: GameState) {
      const notice = state.notices.at(-1) ?? null;
      shown = notice?.id ?? null;
      root.hidden = !notice;
      if (notice) setText(message, notice.message);
    },
  };
}
