/** Every state change the HUD can make. Keeping them here means the input layer and
 *  the components share one vocabulary, and the shape of what the sim and the server
 *  will eventually own stays visible in one file. */

import { spawnerSlotPrice, TILE_CELLS } from '../constants';
import { constrainToAxis, placementRefusal, strokeTiles, toTile } from './build';
import { elementByName, EMPTY_ELEMENT } from '../sim/elements';
import { entityForTool, isToolBuilt } from '../sim/entities';
import { MATERIALS } from './types';
import type { Store } from './store';
import type { DrawerName, GameState, Material, NoticeId, Tool, Vec2 } from './types';

/** What the actions need from the running simulation. */
export interface SimBridge {
  paintTiles(from: Vec2, to: Vec2, element: number): void;
  /** Takes nuggets out of the machines holding them; returns how many it took. */
  spend(amount: number): number;
  elementAt(x: number, y: number): number;
  placeEntity(
    kind: number,
    tileX: number,
    tileY: number,
    element: number,
    width?: number,
    height?: number,
    direction?: number,
  ): boolean;
  entityAt(x: number, y: number): number | null;
  removeEntity(index: number): boolean;
  countOfKind(kind: number): number;
}

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
    /** Selection is instant, no transition. A tool that does nothing yet cannot be
     *  selected — being visibly not built is better than silently ignoring clicks. */
    selectTool(tool: Tool): void {
      if (!isToolBuilt(tool)) return;
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
        case 'press':
        case 'burner':
        case 'compactor':
          this.placeMachine(world);
          return;
        case 'vault':
        case 'belt':
        case 'filter':
          // These are marked out by dragging, and a press is a drag that has not
          // happened yet. `designateVault`/`paintBelt` runs on release.
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
      const { ui } = store.state;
      const machine = entityForTool(ui.selectedTool);
      if (!machine) return;

      // The same rule the ghost previews, so what you see is what happens.
      if (placementRefusal(store.state) !== null) return;

      // An emitter works on the selected material; a press works on whatever it can
      // press, so it carries none.
      let element = EMPTY_ELEMENT;
      if (ui.selectedTool === 'spawner') {
        element = MATERIAL_ELEMENTS[ui.selectedMaterial];
        if (element === undefined) return;
      }

      if (!sim.placeEntity(machine.id, toTile(world.x), toTile(world.y), element)) return;
      this.countMachines();
    },

    /**
     * Declares a region a vault.
     *
     * Storage is something the player builds: dig a pit, wall it, then say that what
     * lands inside counts (spec 5.1). Nothing here checks for walls — physics decides
     * whether the gold stays in, which is the same bargain a press makes.
     */
    designateVault(from: Vec2, to: Vec2): void {
      const machine = entityForTool('vault');
      if (!machine) return;

      const { start, end } = strokeTiles(from, to);
      const x = Math.min(start.x, end.x);
      const y = Math.min(start.y, end.y);
      const width = Math.abs(end.x - start.x) + 1;
      const height = Math.abs(end.y - start.y) + 1;

      sim.placeEntity(machine.id, x, y, EMPTY_ELEMENT, width, height);
    },

    /**
     * Lays a line of belt or filter tiles along a drag.
     *
     * Belts are horizontal only (spec 4's open question on vertical transport is
     * unresolved), so only the drag's x extent matters — the row is wherever it
     * started. Direction is the sign of the drag, resolved once here, matching how a
     * vault's region resolves from start/end on release. A filter additionally reads
     * `selectedMaterial`, the same swatch a spawner reads, for what it lets through.
     */
    paintBelt(from: Vec2, to: Vec2): void {
      const { ui } = store.state;
      const machine = entityForTool(ui.selectedTool);
      if (!machine) return;

      let element = EMPTY_ELEMENT;
      if (ui.selectedTool === 'filter') {
        element = MATERIAL_ELEMENTS[ui.selectedMaterial];
        if (element === undefined) return;
      }

      const { start, end } = strokeTiles(from, to);
      const y = start.y;
      const x0 = Math.min(start.x, end.x);
      const x1 = Math.max(start.x, end.x);
      const direction = end.x < start.x ? -1 : 1;

      for (let x = x0; x <= x1; x += 1) {
        sim.placeEntity(machine.id, x, y, element, 1, 1, direction);
      }
    },

    /**
     * Buys one more spawner slot.
     *
     * The only thing gold does. Capacity, never placement (spec 3.4): where a spawner
     * sits is free to change, and how many you may run is what costs.
     *
     * Paying is physical — nuggets come out of the machines holding them (spec 5.1) —
     * so the sim is the authority on whether the purchase happened. There is no ledger
     * here to disagree with it.
     */
    buySpawnerSlot(): void {
      const price = spawnerSlotPrice(store.state.economy.spawnersMax);
      if (sim.spend(price) !== price) return;

      store.update((state) => ({
        ...state,
        economy: {
          ...state.economy,
          gold: state.economy.gold - price,
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

    /**
     * Reports what the pointer is doing to the world, so the ghost can draw it.
     *
     * `anchor` is where a shift-constrained stroke began — that stroke previews and
     * commits on release, so the ghost is the only thing showing it until then.
     */
    setStroke(anchor: Vec2 | null, painting: boolean): void {
      const { ui } = store.state;
      if (ui.strokeAnchor === anchor && ui.painting === painting) return;
      patchUi({ strokeAnchor: anchor, painting });
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

      // Resolved in tile space, so a constrained stroke lands on the grid like any
      // other.
      const { start, end } = strokeTiles(from, to);
      sim.paintTiles(start, straight ? constrainToAxis(start, end) : end, element);
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
