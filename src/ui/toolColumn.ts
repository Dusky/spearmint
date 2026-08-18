import { MATERIALS, TOOLS } from '../state/types';
import type { Component } from './component';
import type { GameState, Material, Tool } from '../state/types';
import { canBeEmitted, elementByName } from '../sim/elements';
import { TILE_CELLS } from '../constants';
import { el, setClass } from './dom';

interface ToolColumnActions {
  selectTool(tool: Tool): void;
  selectMaterial(material: Material): void;
}

const TOOL_LABELS: Record<Tool, string> = {
  draw: 'Draw',
  erase: 'Erase',
  belt: 'Belt',
  spawner: 'Spawner',
  teleport: 'Teleport',
  blueprint: 'Blueprint',
};

/** Labels and swatch colours both come from the element data, so a new element needs no
 *  change here. */
const MATERIAL_LABELS: Record<Material, string> = Object.fromEntries(
  MATERIALS.map((material) => [material, material]),
) as Record<Material, string>;

export function createToolColumn(actions: ToolColumnActions): Component {
  const toolRows = new Map<Tool, HTMLElement>();
  const swatches = new Map<Material, HTMLElement>();

  const rows = TOOLS.map((tool, index) => {
    const row = el('div', { class: 'tool-row' }, [
      // Glyphs are plain CSS boxes — no icon font, no SVG.
      el('div', { class: `glyph glyph--${tool}` }),
      el('span', {}, [TOOL_LABELS[tool]]),
      // Hotkeys are the row order: 1–6.
      el('span', { class: 'tool-row__hotkey' }, [String(index + 1)]),
    ]);
    row.addEventListener('pointerdown', () => actions.selectTool(tool));
    toolRows.set(tool, row);
    return row;
  });

  const swatchRow = el(
    'div',
    { class: 'swatch-row' },
    MATERIALS.map((material) => {
      const swatch = el('div', { class: 'swatch', title: MATERIAL_LABELS[material] });
      swatch.style.background = elementByName(material).color;
      swatch.addEventListener('pointerdown', () => actions.selectMaterial(material));
      swatches.set(material, swatch);
      return swatch;
    }),
  );

  const caption = el('div', { class: 'material-caption' });

  const root = el('div', { class: 'tool-column' }, [
    el('div', { class: 'section-label' }, ['TOOLS']),
    ...rows,
    el('div', { class: 'tool-column__rule' }),
    el('div', { class: 'section-label' }, ['MATERIAL']),
    swatchRow,
    caption,
    // Load-bearing, and permanent: live particle state is never persisted (spec §7.1),
    // so every machine must be self-starting. The spec requires this be communicated
    // rather than discovered — it is not a dismissible tip.
    el('div', { class: 'self-start-note' }, ['Machines must start themselves. Nothing is primed on load.']),
  ]);

  return {
    root,

    update(state: GameState) {
      for (const [tool, row] of toolRows) {
        setClass(row, 'tool-row--selected', tool === state.ui.selectedTool);
      }
      // An emitter cannot emit a solid, so those swatches read as unavailable while the
      // spawner tool is up rather than silently doing nothing when clicked.
      const emitting = state.ui.selectedTool === 'spawner';
      for (const [material, swatch] of swatches) {
        setClass(swatch, 'swatch--selected', material === state.ui.selectedMaterial);
        setClass(
          swatch,
          'swatch--unavailable',
          emitting && !canBeEmitted(elementByName(material)),
        );
      }
      const label = MATERIAL_LABELS[state.ui.selectedMaterial];
      caption.textContent = `${label} · ${TILE_CELLS}×${TILE_CELLS} snap`;
    },
  };
}
