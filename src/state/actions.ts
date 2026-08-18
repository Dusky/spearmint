/** Every state change the HUD can make. Keeping them here means the input layer and
 *  the components share one vocabulary, and the shape of what the sim and the server
 *  will eventually own stays visible in one file. */

import { spawnerSlotPrice, TILE_CELLS } from '../constants';
import { canBeEmitted, elementByName, EMPTY_ELEMENT } from '../sim/elements';
import { entityForTool } from '../sim/entities';
import { MATERIALS } from './types';
import type { Store } from './store';
import type { DrawerName, GameState, Material, NoticeId, Tool, Vec2 } from './types';

/** What the actions need from the running simulation. */
export interface SimBridge {
  paintTiles(from: Vec2, to: Vec2, element: number): void;
  elementAt(x: number, y: number): number;
  placeEntity(kind: number, tileX: number, tileY: number, element: number): boolean;
  entityAt(x: number, y: number): number | null;
  removeEntity(index: number): boolean;
  countOfKind(kind: number): number;
}

const toTile = (cell: number): number => Math.floor(cell / TILE_CELLS);

/** Materials are element names, so these resolve straight out of the data file. */
const MATERIAL_ELEMENTS: Record<Material, number> = Object.fromEntries(
  MATERIALS.map((material) => [material, elementByName(material).id]),
) as Record<Material, number>;

const MATERIAL_BY_ELEMENT = new Map<number, Material>(
  MATERIALS.map((material) => [MATERIAL_ELEMENTS[material], material]),
);

export function createActions(store: Store<GameState>, sim: SimBridge) {
  const patchUi = (patch: Partial<GameState['ui']>): void => {
    store.update((state) => ({ ...state, ui: { ...state.ui, ...patch } }));
  };

  return {
    /** Selection is instant, no transition. */
    selectTool(tool: Tool): void {
      patchUi({ selectedTool: tool });
    },

    selectMaterial(material: Material): void {
      patchUi({ selectedMaterial: material });
    },

    moveCursor(cursor: Vec2): void {
      const current = store.state.ui.cursor;
      if (current.x === cursor.x && current.y === cursor.y) return;
      patchUi({ cursor });
    },

    panBy(dx: number, dy: number): void {
      const { camera } = store.state.ui;
      patchUi({ camera: { ...camera, x: camera.x + dx, y: camera.y + dy } });
    },

    panTo(at: Vec2): void {
      const { camera } = store.state.ui;
      patchUi({ camera: { ...camera, x: at.x, y: at.y } });
    },

    /**
     * A press in the viewport, dispatched by tool.
     *
     * Kept here rather than in the viewport so the viewport stays a pointer-to-world
     * translator and every tool's meaning lives in one file.
     */
    press(world: Vec2): void {
      switch (store.state.ui.selectedTool) {
        case 'spawner':
        case 'collector':
          this.placeMachine(world);
          return;
        case 'erase':
          // Erase should erase. Removing an entity here rather than inventing another
          // tool, and on press only — never mid-drag, so sweeping erase across the
          // world cannot silently delete a machine's inputs.
          if (this.removeEntityAt(world)) return;
          this.paint(world, world, false);
          return;
        case 'draw':
          // A click is a stroke that never moved, and it should leave a tile. Painting
          // only happens on drag otherwise, so clicking would do nothing at all.
          this.paint(world, world, false);
          return;
        default:
          break;
      }
      this.selectAt(world);
    },

    /**
     * Places whatever machine the selected tool builds.
     *
     * One path for every machine: the tool names it and the data says the rest, which
     * is the whole point of entities being a table rather than a set of special cases.
     * The only tool-specific rule is the spawner cap — spawner count is the game's one
     * hard limit on production (spec 3.4), so this refuses past it rather than letting
     * the player buy their way out with geometry.
     */
    placeMachine(world: Vec2): void {
      const { economy, ui } = store.state;
      const machine = entityForTool(ui.selectedTool);
      if (!machine) return;

      const emitting = ui.selectedTool === 'spawner';
      if (emitting && economy.spawnersOwned >= economy.spawnersMax) return;

      // An emitter works on the selected material; a collector eats whatever falls in,
      // so it carries none.
      let element = EMPTY_ELEMENT;
      if (emitting) {
        element = MATERIAL_ELEMENTS[ui.selectedMaterial];
        if (element === undefined) return;
        // A wall emitter is nonsense: solids do not flow, so there is nothing to emit.
        if (!canBeEmitted(elementByName(ui.selectedMaterial))) return;
      }

      if (!sim.placeEntity(machine.id, toTile(world.x), toTile(world.y), element)) return;
      this.countMachines();
    },

    /**
     * Buys one more spawner slot.
     *
     * The only thing gold does. Capacity, never placement (spec 3.4): where a spawner
     * sits is free to change, and how many you may run is what costs.
     */
    buySpawnerSlot(): void {
      const { economy } = store.state;
      const price = spawnerSlotPrice(economy.spawnersMax);
      if (economy.gold < price) return;

      store.update((state) => ({
        ...state,
        economy: {
          ...state.economy,
          gold: state.economy.gold - price,
          spent: state.economy.spent + price,
          spawnersMax: state.economy.spawnersMax + 1,
        },
      }));
    },

    /** Re-reads how many spawners exist. The sim is the register; the store mirrors it. */
    countMachines(): void {
      const spawner = entityForTool('spawner');
      if (!spawner) return;
      const spawnersOwned = sim.countOfKind(spawner.id);
      store.update((state) => ({
        ...state,
        economy: { ...state.economy, spawnersOwned },
      }));
    },

    /**
     * Removes whatever entity sits here, refunding its slot in full.
     *
     * Placement is never a purchase that can be wasted. Gold buys spawner *capacity*
     * (spec 3.4); where a spawner sits within that capacity is a layout decision, and
     * taking it back costs nothing. Without this, spending every slot on one material
     * soft-locks the run.
     */
    removeEntityAt(world: Vec2): boolean {
      const index = sim.entityAt(world.x, world.y);
      if (index === null) return false;

      sim.removeEntity(index);
      this.countMachines();
      return true;
    },

    /** Click a construct to select it; clicking past everything clears the selection. */
    selectAt(world: Vec2): void {
      const hit = store.state.constructs.find(({ bounds }) => {
        const left = bounds.x * TILE_CELLS;
        const top = bounds.y * TILE_CELLS;
        return (
          world.x >= left &&
          world.x < left + bounds.width * TILE_CELLS &&
          world.y >= top &&
          world.y < top + bounds.height * TILE_CELLS
        );
      });
      patchUi({ selection: hit?.id ?? null });
    },

    clearSelection(): void {
      patchUi({ selection: null });
    },

    toggleTileGrid(): void {
      patchUi({ showTileGrid: !store.state.ui.showTileGrid });
    },

    /** Only one drawer is open at a time; asking for the open one closes it. */
    toggleDrawer(drawer: DrawerName): void {
      patchUi({ openDrawer: store.state.ui.openDrawer === drawer ? null : drawer });
    },

    closeDrawer(): void {
      patchUi({ openDrawer: null });
    },

    dismissNotice(id: NoticeId): void {
      store.update((state) => ({
        ...state,
        notices: state.notices.filter((notice) => notice.id !== id),
      }));
    },

    /** `F` pans the camera to the most recent notice. It does not dismiss it. */
    jumpToNotice(): void {
      const notice = store.state.notices.at(-1);
      if (!notice) return;
      const { camera } = store.state.ui;
      patchUi({ camera: { ...camera, x: notice.at.x, y: notice.at.y } });
    },

    /**
     * Draws a stroke into the world.
     *
     * Everything the player builds sits on the tile grid (spec 2.3), so a stroke is a
     * line over *tiles* and each one it touches is filled completely. The tile is the
     * brush; there is no partial tile. Holding shift constrains the stroke to whichever
     * axis it has travelled furthest along.
     */
    paint(from: Vec2, to: Vec2, straight: boolean): void {
      const { selectedTool, selectedMaterial } = store.state.ui;
      if (selectedTool !== 'draw' && selectedTool !== 'erase') return;

      const element =
        selectedTool === 'erase' ? EMPTY_ELEMENT : MATERIAL_ELEMENTS[selectedMaterial];
      if (element === undefined) return;

      const start = { x: toTile(from.x), y: toTile(from.y) };
      let end = { x: toTile(to.x), y: toTile(to.y) };
      if (straight) {
        // Resolved in tile space, so a constrained stroke lands on the grid like any
        // other.
        end =
          Math.abs(end.x - start.x) >= Math.abs(end.y - start.y)
            ? { x: end.x, y: start.y }
            : { x: start.x, y: end.y };
      }

      sim.paintTiles(start, end, element);
    },

    /** alt+click: adopt the material already under the cursor. */
    pickMaterialAt(world: Vec2): void {
      const element = sim.elementAt(world.x, world.y);
      const material = MATERIAL_BY_ELEMENT.get(element);
      if (material) patchUi({ selectedMaterial: material });
    },
  };
}

export type Actions = ReturnType<typeof createActions>;
