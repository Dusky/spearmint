import { MATERIALS, TOOLS, TOOL_GROUPS } from '../state/types';
import type { Component } from './component';
import type { GameState, Material, Tool } from '../state/types';
import { canBeEmitted, elementByName, FILTER_TARGETS } from '../sim/elements';
import { isToolBuilt } from '../sim/entities';
import { TILE_CELLS } from '../constants';
import { createProperties } from './properties';
import { el, setClass } from './dom';
import type { PropertiesActions } from './properties';

interface ToolColumnActions extends PropertiesActions {
  selectTool(tool: Tool): void;
  selectMaterial(material: Material): void;
  selectFilterTarget(elementName: string): void;
}

const TOOL_LABELS: Record<Tool, string> = {
  select: 'Select',
  draw: 'Draw',
  erase: 'Erase',
  belt: 'Belt',
  filter: 'Filter',
  kiln: 'Kiln',
  lift: 'Lift',
  spawner: 'Spawner',
  press: 'Press',
  heater: 'Heater',
  vault: 'Vault',
  teleport: 'Teleport',
  blueprint: 'Blueprint',
};

/** One line each, shown as the row's tooltip. Says what the tool is *for*, which the
 *  label alone cannot — "Filter" and "Press" are nouns until you know the mechanic. */
const TOOL_HINTS: Record<Tool, string> = {
  select: 'Click a machine to inspect and retune it',
  draw: 'Paint the selected material, a tile at a time',
  erase: 'Clear tiles, or remove a machine you click',
  belt: 'Drag out a run; it carries whatever rests on top',
  filter: 'A belt that drops one chosen element through its underside',
  kiln: 'A belt that conducts. Heat it, and it cooks what it carries',
  lift: 'Drag out a shaft; it carries upward. Feed it with a belt underneath',
  spawner: 'Introduces matter. The only hard limit on production',
  press: 'Presses product into gold nuggets, and leaves residue',
  heater: 'Burns fuel to heat whatever touches it',
  vault: 'Drag out a region; gold inside it counts as money',
  teleport: 'Not built yet',
  blueprint: 'Not built yet',
};

/** Labels and swatch colours both come from the element data, so a new element needs no
 *  change here. */
const MATERIAL_LABELS: Record<Material, string> = Object.fromEntries(
  MATERIALS.map((material) => [material, material]),
) as Record<Material, string>;

export function createToolColumn(actions: ToolColumnActions): Component {
  const toolRows = new Map<Tool, HTMLElement>();
  const swatches = new Map<Material, HTMLElement>();
  const filterSwatches = new Map<string, HTMLElement>();
  const properties = createProperties(actions);

  const toolRow = (tool: Tool): HTMLElement => {
    // Unbuilt tools are shown rather than hidden — the shape of the game is part of the
    // design — but they do not respond, because a tool that silently ignores clicks is
    // worse than one that is visibly not finished.
    const built = isToolBuilt(tool);
    const row = el('div', { class: built ? 'tool-row' : 'tool-row tool-row--unbuilt' }, [
      // Glyphs are plain CSS boxes — no icon font, no SVG.
      el('div', { class: `glyph glyph--${tool}` }),
      el('span', {}, [TOOL_LABELS[tool]]),
      // Hotkeys are the tool's position in TOOLS, not its position within its group,
      // so grouping stays purely visual and the keys keep matching the whole list.
      el('span', { class: 'tool-row__hotkey' }, [String(TOOLS.indexOf(tool) + 1)]),
    ]);
    row.title = built ? TOOL_HINTS[tool] : 'Not built yet';
    row.addEventListener('pointerdown', () => actions.selectTool(tool));
    toolRows.set(tool, row);
    return row;
  };

  // Grouped rather than one flat list: thirteen tools in a single column is a wall to
  // scan, and the groups are the questions you are actually asking of it.
  const groups = TOOL_GROUPS.flatMap((group) => [
    el('div', { class: 'section-label section-label--group' }, [group.label]),
    ...group.tools.map(toolRow),
  ]);

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

  // A filter's target is the full element roster (`FILTER_TARGETS`), not the curated
  // three `MATERIALS` offers — it only ever routes matter that already exists, so
  // letting it target gold or fuel cannot hand the player anything for free.
  const filterSwatchRow = el(
    'div',
    { class: 'swatch-row' },
    FILTER_TARGETS.map((element) => {
      const swatch = el('div', { class: 'swatch', title: element.name });
      swatch.style.background = element.color;
      swatch.addEventListener('pointerdown', () => actions.selectFilterTarget(element.name));
      filterSwatches.set(element.name, swatch);
      return swatch;
    }),
  );

  const sectionLabel = el('div', { class: 'section-label' }, ['MATERIAL']);
  const caption = el('div', { class: 'material-caption' });

  // Only the tools and swatches scroll. With thirteen tools the column outgrew a short
  // viewport, but the properties panel is exactly what you are looking at while you
  // fiddle with a machine, so it stays pinned rather than sitting below the fold.
  const scroller = el('div', { class: 'tool-column__scroll' }, [
    ...groups,
    el('div', { class: 'tool-column__rule' }),
    sectionLabel,
    swatchRow,
    filterSwatchRow,
    caption,
  ]);

  const root = el('div', { class: 'tool-column' }, [
    scroller,
    properties.root,
    // Load-bearing, and permanent: live particle state is never persisted (spec §7.1),
    // so every machine must be self-starting. The spec requires this be communicated
    // rather than discovered — it is not a dismissible tip.
    el('div', { class: 'self-start-note' }, ['Machines must start themselves. Nothing is primed on load.']),
  ]);

  return {
    root,

    update(state: GameState) {
      properties.update(state);
      for (const [tool, row] of toolRows) {
        setClass(row, 'tool-row--selected', tool === state.ui.selectedTool);
      }

      const isFilter = state.ui.selectedTool === 'filter';
      sectionLabel.textContent = isFilter ? 'FILTER TARGET' : 'MATERIAL';
      swatchRow.hidden = isFilter;
      filterSwatchRow.hidden = !isFilter;

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
      for (const [name, swatch] of filterSwatches) {
        setClass(swatch, 'swatch--selected', name === state.ui.filterTarget);
      }

      caption.textContent = isFilter
        ? `${state.ui.filterTarget} · lets it fall through`
        : `${MATERIAL_LABELS[state.ui.selectedMaterial]} · ${TILE_CELLS}×${TILE_CELLS} snap`;
    },
  };
}
