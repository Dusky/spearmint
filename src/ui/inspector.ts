import { MEASUREMENTS, deviationOf, diagnose } from '../state/measurements';
import { formatPercent } from './format';
import { el, setClass, setText } from './dom';
import type { Component } from './component';
import type { GameState } from '../state/types';

/**
 * The single opinionated panel. Reaction yield varies with layout rather than fixed
 * ratios (spec §3.3), so the interface's job is to explain *why* a machine
 * underperforms: it reports physical conditions and names the bottleneck in a
 * sentence. It never reports an abstracted throughput number — the spec forbids it
 * (§1.1, §6).
 */
export function createInspector(): Component {
  const name = el('span', { class: 'inspector__name' });
  const status = el('span', { class: 'inspector__status' });
  const best = el('span', { class: 'inspector__yield-best' });

  const yieldNumber = document.createTextNode('');
  const yieldValue = el('span', { class: 'inspector__yield-value' }, [
    yieldNumber,
    el('span', { class: 'inspector__yield-unit' }, ['%']),
  ]);

  const barFill = el('div', { class: 'inspector__bar-fill' });
  const barBest = el('div', { class: 'inspector__bar-best' });

  const rows = MEASUREMENTS.map((spec) => {
    const value = el('span', { class: 'measurement__value' });
    const row = el('div', { class: 'measurement' }, [
      el('span', { class: 'measurement__key' }, [spec.label]),
      value,
    ]);
    return { spec, row, value };
  });

  const diagnosis = el('div', { class: 'inspector__diagnosis' });

  const root = el('div', { class: 'inspector' }, [
    el('div', { class: 'inspector__header' }, [name, status]),
    el('div', { class: 'inspector__yield' }, [
      el('div', { class: 'inspector__yield-head' }, [
        el('span', { class: 'field-label' }, ['YIELD']),
        best,
      ]),
      yieldValue,
      el('div', { class: 'inspector__bar' }, [barFill, barBest]),
    ]),
    el('div', { class: 'inspector__rule' }),
    el('div', { class: 'inspector__measurements' }, rows.map((row) => row.row)),
    el('div', { class: 'inspector__rule' }),
    diagnosis,
  ]);

  return {
    root,

    update(state: GameState) {
      const construct = state.constructs.find((item) => item.id === state.ui.selection);
      const readout = state.readout;
      // The inspector exists only for a selection.
      root.hidden = !construct || !readout || readout.constructId !== construct.id;
      if (!construct || !readout || root.hidden) return;

      setText(name, construct.name);
      setText(status, construct.running ? 'running' : 'stalled');
      setClass(status, 'inspector__status--stalled', !construct.running);

      // "Best seen" is a per-blueprint high-water mark, so it is read from the
      // blueprint rather than from this instance.
      const blueprint = state.blueprints.find((item) => item.id === construct.blueprintId);
      const yieldBest = blueprint?.yieldBest ?? null;

      setText(best, yieldBest === null ? '' : `best seen ${formatPercent(yieldBest)}%`);
      setText(yieldNumber, formatPercent(readout.yieldCurrent));
      barFill.style.width = `${(readout.yieldCurrent * 100).toFixed(2)}%`;
      barBest.hidden = yieldBest === null;
      if (yieldBest !== null) barBest.style.left = `${(yieldBest * 100).toFixed(2)}%`;

      for (const { spec, value } of rows) {
        const reading = readout[spec.key];
        setText(value, spec.format(reading));
        // The only highlight in the panel. No icon, no badge.
        setClass(value, 'measurement__value--out-of-range', deviationOf(spec, reading) !== null);
      }

      setText(diagnosis, diagnose(readout));
    },
  };
}
