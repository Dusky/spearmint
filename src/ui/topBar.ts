import { formatInteger } from './format';
import { el, setClass, setText } from './dom';
import { SPEEDS } from '../constants';
import type { Actions } from '../state/actions';
import type { Component } from './component';
import type { GameState } from '../state/types';

/** What each speed reads as. Pause gets a glyph rather than `0x`, because it is a state
 *  the player is switching into and not a rate they are choosing. */
function speedLabel(speed: number): string {
  return speed === 0 ? '❚❚' : `${speed}x`;
}

/**
 * Economy readouts. Spawner count sits here next to gold, rather than in a menu,
 * because it is the game's only hard input cap (spec §1.1, §3.4).
 *
 * The view controls — speed and the heat overlay — sit at the other end, beside the
 * tick rate they qualify. Neither changes the world: one decides how often the host
 * asks for a tick (spec 3.1 keeps the timestep out of the sim), the other decides how
 * cells are coloured.
 */
export function createTopBar(actions: Actions): Component {
  const goldValue = el('span', { class: 'gold-value' });
  const goldRate = el('span', { class: 'gold-rate' });

  const pips: HTMLElement[] = [];
  const pipRow = el('div', { class: 'spawner-pips' });
  const spawnerCount = el('span', { class: 'spawner-count' });

  const tick = el('span');
  const seed = el('span');

  const speedButtons = SPEEDS.map((speed) => {
    const button = el(
      'button',
      {
        type: 'button',
        class: 'speed-button',
        title: speed === 0 ? 'pause (p)' : `run at ${speed}x`,
      },
      [speedLabel(speed)],
    );
    button.addEventListener('click', () => actions.setSpeed(speed));
    return { speed, button };
  });

  const heatButton = el(
    'button',
    { type: 'button', class: 'view-toggle', title: 'tint cells by temperature (h)' },
    ['HEAT'],
  );
  heatButton.addEventListener('click', () => actions.toggleHeatOverlay());

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
      el(
        'div',
        { class: 'speed-buttons' },
        speedButtons.map(({ button }) => button),
      ),
      heatButton,
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
      setText(seed, `seed ${state.seed.toString(16)}`);

      const { speed, showHeat } = state.ui;
      for (const button of speedButtons) {
        setClass(button.button, 'speed-button--active', button.speed === speed);
      }
      setClass(heatButton, 'view-toggle--active', showHeat);

      // The effective rate, not the configured one — a paused sim reporting 30 Hz would
      // be reporting a number nothing is running at.
      if (tickRate) {
        setText(tickRate, speed === 0 ? 'paused' : `${state.tickRate * speed} Hz`);
      }
      setClass(root, 'top-bar--paused', speed === 0);
    },
  };
}
