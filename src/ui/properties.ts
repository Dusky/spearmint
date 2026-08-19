import { entityByKind, hasRate } from '../sim/entities';
import { el, setClass, setText } from './dom';
import type { Component } from './component';
import type { GameState } from '../state/types';

export interface PropertiesActions {
  setSelectedEnabled(enabled: boolean): void;
  setSelectedRate(rate: number): void;
  removeSelected(): void;
}

/**
 * What the selected machine is, and the settings you can change without replacing it.
 *
 * Placement used to be the only moment you could decide anything about a machine, so
 * changing your mind meant removing it and building it again — which loses where it was
 * pointed and, for an emitter, costs the slot in between. Everything here is a property
 * of one placed instance, not of its kind.
 *
 * Hidden entirely with nothing selected. An empty panel of disabled controls says less
 * than no panel at all, and the tool column is already competing for the space.
 */
export function createProperties(actions: PropertiesActions): Component {
  const name = el('div', { class: 'properties__name' });
  const state = el('span', { class: 'properties__state' });

  const power = el('button', { class: 'properties__power', type: 'button' });
  power.addEventListener('click', () => actions.setSelectedEnabled(!enabled));

  const rateValue = el('span', { class: 'properties__rate-value' });
  const rate = el('input', {
    class: 'properties__slider',
    type: 'range',
    min: '1',
    max: '12',
    step: '1',
  }) as HTMLInputElement;
  rate.addEventListener('input', () => actions.setSelectedRate(Number(rate.value)));

  const rateRow = el('label', { class: 'properties__row' }, [
    el('span', { class: 'field-label' }, ['RATE']),
    rateValue,
    rate,
  ]);

  const remove = el('button', { class: 'properties__remove', type: 'button' }, ['Remove']);
  remove.addEventListener('click', () => actions.removeSelected());

  const root = el('div', { class: 'properties' }, [
    el('div', { class: 'section-label section-label--group' }, ['SELECTED']),
    el('div', { class: 'properties__header' }, [name, state]),
    power,
    rateRow,
    remove,
  ]);

  /** Mirrors the switch's own state, so the click handler knows what to flip to. */
  let enabled = true;

  return {
    root,

    update(gameState: GameState) {
      const selected = gameState.ui.selectedEntity;
      root.hidden = !selected;
      if (!selected) return;

      const machine = entityByKind(selected.kind);
      enabled = selected.enabled;

      setText(name, machine?.name ?? 'machine');
      setText(state, enabled ? 'running' : 'off');
      setClass(state, 'properties__state--off', !enabled);

      setText(power, enabled ? 'Switch off' : 'Switch on');
      setClass(power, 'properties__power--off', !enabled);

      // Only machines whose throughput is actually read get a slider. Belts and vaults
      // carry a `rate` in the data because the field defaults, but nothing consults it,
      // so a control for it would be a lie.
      const tunable = machine ? hasRate(machine) : false;
      rateRow.hidden = !tunable;
      if (tunable) {
        setText(rateValue, String(selected.rate));
        // Only write while the player is not dragging it — assigning `value` mid-drag
        // fights the thumb and makes the slider stutter.
        if (document.activeElement !== rate) rate.value = String(selected.rate);
      }
    },
  };
}
