import { spawnerSlotPrice } from '../../constants';
import { formatInteger } from '../format';
import { el, setClass, setText } from '../dom';
import type { Component } from '../component';
import type { GameState, UpgradeId } from '../../state/types';

interface CapabilityActions {
  buySpawnerSlot(): void;
}

/**
 * Gold buys capability; raw material builds machines (spec §5.2).
 *
 * One row is real: spawner capacity. It is the game's only hard cap on production
 * (spec §3.4), which makes it the only thing worth spending on this early — and a
 * drawer with one purchase that works beats one with three that do not.
 *
 * In-game this is an overlay drawer, not a panel in a second row — the prototype laid
 * the supporting panels out flat only so every state was visible at once.
 */
export function createCapabilityDrawer(actions: CapabilityActions): Component {
  const list = el('div', { class: 'capability__list' });

  const root = el('div', { class: 'drawer capability' }, [
    el('div', { class: 'drawer__title' }, ['Capability']),
    el('div', { class: 'drawer__subtitle' }, ['Gold buys reach. Raw material builds machines.']),
    el('div', { class: 'drawer__rule' }),
    list,
  ]);

  const slotTitle = document.createTextNode('');
  const slotPrice = document.createTextNode('');
  const slotRow = el('div', { class: 'capability-row capability-row--action' }, [
    el('div', { class: 'capability-row__text' }, [
      el('div', { class: 'capability-row__title' }, [slotTitle]),
      el('div', { class: 'capability-row__description' }, [
        'The only thing that raises your ceiling.',
      ]),
    ]),
    el('div', { class: 'capability-row__price' }, [slotPrice]),
  ]);
  slotRow.addEventListener('pointerdown', () => actions.buySpawnerSlot());
  list.append(slotRow);

  const rows = new Map<UpgradeId, { row: HTMLElement; title: Text; description: Text; price: Text }>();

  return {
    root,

    update(state: GameState) {
      root.hidden = state.ui.openDrawer !== 'capability';
      if (root.hidden) return;

      const { spawnersMax, gold } = state.economy;
      const price = spawnerSlotPrice(spawnersMax);
      setText(slotTitle, `Spawner ${spawnersMax} → ${spawnersMax + 1}`);
      setText(slotPrice, formatInteger(price));
      // Unaffordable is the whole row at .45 — not greyed text, not disabled styling.
      setClass(slotRow, 'capability-row--unaffordable', gold < price);

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
        setClass(entry.row, 'capability-row--unaffordable', state.economy.gold < upgrade.price);
      }
    },
  };
}
