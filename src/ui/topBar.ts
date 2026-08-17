import { formatInteger } from './format';
import { el, setClass, setText } from './dom';
import type { Component } from './component';
import type { GameState } from '../state/types';

/**
 * Economy readouts. Spawner count sits here next to gold, rather than in a menu,
 * because it is the game's only hard input cap (spec §1.1, §3.4).
 */
export function createTopBar(): Component {
  const goldValue = el('span', { class: 'gold-value' });
  const goldRate = el('span', { class: 'gold-rate' });

  const pips: HTMLElement[] = [];
  const pipRow = el('div', { class: 'spawner-pips' });
  const spawnerCount = el('span', { class: 'spawner-count' });

  const tick = el('span');
  const seed = el('span');

  const root = el('div', { class: 'top-bar' }, [
    el('div', { class: 'top-bar__group top-bar__group--gold' }, [
      el('span', { class: 'field-label' }, ['GOLD']),
      goldValue,
      goldRate,
    ]),
    el('div', { class: 'top-bar__divider' }),
    el('div', { class: 'top-bar__group' }, [
      el('span', { class: 'field-label' }, ['SPAWNERS']),
      pipRow,
      spawnerCount,
    ]),
    el('div', { class: 'top-bar__spacer' }),
    el('div', { class: 'top-bar__status' }, [
      tick,
      seed,
      el('div', { class: 'tick-rate' }, [el('span', { class: 'tick-rate__dot' }), el('span', {}, [''])]),
    ]),
  ]);

  const tickRate = root.querySelector('.tick-rate > span:last-child');

  return {
    root,

    update(state: GameState) {
      const { economy } = state;

      setText(goldValue, formatInteger(economy.gold));
      setText(goldRate, `+${formatInteger(economy.goldRate)}/s`);

      // The pip row is the spawner cap made visible: one rect per possible spawner.
      while (pips.length > economy.spawnersMax) pips.pop()?.remove();
      while (pips.length < economy.spawnersMax) {
        const pip = el('div', { class: 'spawner-pip' });
        pips.push(pip);
        pipRow.append(pip);
      }
      pips.forEach((pip, index) => setClass(pip, 'spawner-pip--owned', index < economy.spawnersOwned));
      setText(spawnerCount, `${economy.spawnersOwned} / ${economy.spawnersMax}`);

      setText(tick, `tick ${formatInteger(state.tick)}`);
      setText(seed, `seed ${state.seed}`);
      if (tickRate) setText(tickRate, `${state.tickRate} Hz`);
    },
  };
}
