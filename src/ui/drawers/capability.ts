import { formatInteger } from '../format';
import { el, setClass, setText } from '../dom';
import type { Component } from '../component';
import type { GameState, UpgradeId } from '../../state/types';

/**
 * Gold buys capability; raw material builds machines (spec §5.2). Rows are read-only
 * here: purchasing is server-owned (spec §8.1) and the transaction UI is undesigned.
 *
 * In-game this is an overlay drawer, not a panel in a second row — the prototype laid
 * the supporting panels out flat only so every state was visible at once.
 */
export function createCapabilityDrawer(): Component {
  const list = el('div', { class: 'capability__list' });

  const root = el('div', { class: 'drawer capability' }, [
    el('div', { class: 'drawer__title' }, ['Capability']),
    el('div', { class: 'drawer__subtitle' }, ['Gold buys reach. Raw material builds machines.']),
    el('div', { class: 'drawer__rule' }),
    list,
  ]);

  const rows = new Map<UpgradeId, { row: HTMLElement; title: Text; description: Text; price: Text }>();

  return {
    root,

    update(state: GameState) {
      root.hidden = state.ui.openDrawer !== 'capability';
      if (root.hidden) return;

      for (const upgrade of state.upgrades) {
        let entry = rows.get(upgrade.id);
        if (!entry) {
          const title = document.createTextNode('');
          const description = document.createTextNode('');
          const price = document.createTextNode('');
          const row = el('div', { class: 'capability-row' }, [
            el('div', { class: 'capability-row__text' }, [
              el('div', { class: 'capability-row__title' }, [title]),
              el('div', { class: 'capability-row__description' }, [description]),
            ]),
            el('div', { class: 'capability-row__price' }, [price]),
          ]);
          entry = { row, title, description, price };
          rows.set(upgrade.id, entry);
          list.append(row);
        }

        setText(entry.title, upgrade.title);
        setText(entry.description, upgrade.description);
        setText(entry.price, formatInteger(upgrade.price));
        // Unaffordable is the whole row at .45 — not greyed text, not disabled styling.
        setClass(entry.row, 'capability-row--unaffordable', state.economy.gold < upgrade.price);
      }
    },
  };
}
