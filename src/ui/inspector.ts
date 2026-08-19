import { diagnose, measurementsOf } from '../state/measurements';
import { formatPercent } from './format';
import { el, setClass, setText } from './dom';
import type { Component } from './component';
import type { GameState } from '../state/types';

/**
 * The single opinionated panel. Reaction yield varies with layout rather than fixed
 * ratios (spec 3.3), so the interface's job is to explain *why* the factory is doing
 * what it is doing, and it never reports an abstracted throughput number (spec 1.1, 6).
 *
 * Every row here is measured. The design handoff also shows temperature, residence and
 * mixing; temperature arrived with the heat system, and the other two are still absent
 * because nothing computes them yet — inventing numbers to fill the panel would be
 * worse than a shorter one.
 *
 * Measurements are whole-world. Per-machine selection is the real design — "why is
 * *this* machine underperforming" — and needs a way to mark out a machine first.
 */
export function createInspector(): Component {
  const status = el('span', { class: 'inspector__status' });
  const best = el('span', { class: 'inspector__yield-best' });

  const yieldNumber = document.createTextNode('');
  const yieldValue = el('span', { class: 'inspector__yield-value' }, [
    yieldNumber,
    el('span', { class: 'inspector__yield-unit' }, ['%']),
  ]);

  const barFill = el('div', { class: 'inspector__bar-fill' });
  const barBest = el('div', { class: 'inspector__bar-best' });

  const rows = el('div', { class: 'inspector__measurements' });
  const diagnosis = el('div', { class: 'inspector__diagnosis' });

  const root = el('div', { class: 'inspector' }, [
    el('div', { class: 'inspector__header' }, [
      el('span', { class: 'inspector__name' }, ['Factory']),
      status,
    ]),
    el('div', { class: 'inspector__yield' }, [
      el('div', { class: 'inspector__yield-head' }, [
        el('span', { class: 'field-label' }, ['YIELD']),
        best,
      ]),
      yieldValue,
      el('div', { class: 'inspector__bar' }, [barFill, barBest]),
    ]),
    el('div', { class: 'inspector__rule' }),
    rows,
    el('div', { class: 'inspector__rule' }),
    diagnosis,
  ]);

  // Best yield seen, kept here because it is a property of the session rather than of
  // the world. A per-blueprint high-water mark is the real design (spec 4.5).
  let bestSeen = 0;
  const cells = new Map<string, { key: Text; value: HTMLElement }>();

  return {
    root,

    update(state: GameState) {
      const readout = state.readout;
      root.hidden = !readout;
      if (!readout) return;

      const running = readout.sand > 0 || readout.water > 0;
      setText(status, running ? 'running' : 'idle');
      setClass(status, 'inspector__status--stalled', !running);

      bestSeen = Math.max(bestSeen, readout.yieldCurrent);
      setText(best, bestSeen > 0 ? `best seen ${formatPercent(bestSeen)}%` : '');
      setText(yieldNumber, formatPercent(readout.yieldCurrent));
      barFill.style.width = `${(readout.yieldCurrent * 100).toFixed(2)}%`;
      barBest.hidden = bestSeen <= 0;
      barBest.style.left = `${(bestSeen * 100).toFixed(2)}%`;

      for (const measurement of measurementsOf(readout)) {
        let cell = cells.get(measurement.label);
        if (!cell) {
          const key = document.createTextNode(measurement.label);
          const value = el('span', { class: 'measurement__value' });
          rows.append(
            el('div', { class: 'measurement' }, [
              el('span', { class: 'measurement__key' }, [key]),
              value,
            ]),
          );
          cell = { key, value };
          cells.set(measurement.label, cell);
        }
        setText(cell.value, measurement.value);
        // The only highlight in the panel: this reading is the reason for the ceiling.
        setClass(cell.value, 'measurement__value--out-of-range', measurement.limiting);
      }

      setText(diagnosis, diagnose(state));
    },
  };
}
