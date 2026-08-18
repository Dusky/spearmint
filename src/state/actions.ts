/** Every state change the HUD can make. Keeping them here means the input layer and
 *  the components share one vocabulary, and the shape of what the sim and the server
 *  will eventually own stays visible in one file. */

import { TILE_CELLS } from '../constants';
import { elementByName, EMPTY_ELEMENT } from '../sim/elements';
import { MATERIALS } from './types';
import type { Store } from './store';
import type { DrawerName, GameState, Material, NoticeId, Tool, Vec2 } from './types';

/** What the actions need from the running simulation. */
export interface SimBridge {
  paintLine(from: Vec2, to: Vec2, element: number, halfWidth: number): void;
  elementAt(x: number, y: number): number;
}

/** Materials are element names, so these resolve straight out of the data file. */
const MATERIAL_ELEMENTS: Record<Material, number> = Object.fromEntries(
  MATERIALS.map((material) => [material, elementByName(material).id]),
) as Record<Material, number>;

const MATERIAL_BY_ELEMENT = new Map<number, Material>(
  MATERIALS.map((material) => [MATERIAL_ELEMENTS[material], material]),
);

/** Cells from a tile's origin to its centre. Tiles are odd-sized so this is exact. */
const TILE_CENTRE = (TILE_CELLS - 1) / 2;

/** Snaps a world cell to the centre of the tile containing it. */
function snapToTileCentre(cell: number): number {
  return Math.floor(cell / TILE_CELLS) * TILE_CELLS + TILE_CENTRE;
}

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
     * Building happens on the tile grid, not per cell (spec 2.3), so both ends snap to
     * tile centres and the brush is one tile across. Holding shift constrains the
     * stroke to the axis it has travelled furthest along.
     */
    paint(from: Vec2, to: Vec2, straight: boolean): void {
      const { selectedTool, selectedMaterial } = store.state.ui;
      if (selectedTool !== 'draw' && selectedTool !== 'erase') return;

      const element =
        selectedTool === 'erase' ? EMPTY_ELEMENT : MATERIAL_ELEMENTS[selectedMaterial];
      if (element === undefined) return;

      let end = to;
      if (straight) {
        end =
          Math.abs(to.x - from.x) >= Math.abs(to.y - from.y)
            ? { x: to.x, y: from.y }
            : { x: from.x, y: to.y };
      }

      sim.paintLine(
        { x: snapToTileCentre(from.x), y: snapToTileCentre(from.y) },
        { x: snapToTileCentre(end.x), y: snapToTileCentre(end.y) },
        element,
        TILE_CENTRE,
      );
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
