/** Every state change the HUD can make. Keeping them here means the input layer and
 *  the components share one vocabulary, and the shape of what the sim and the server
 *  will eventually own stays visible in one file. */

import { TILE_CELLS } from '../constants';
import type { Store } from './store';
import type { DrawerName, GameState, Material, NoticeId, Tool, Vec2 } from './types';

export function createActions(store: Store<GameState>) {
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
     * SEAM — needs the sim.
     *
     * Drawing paints at the tile snap, and shift constrains the stroke to a straight
     * line. Both are settled in the design, but a stroke has nowhere to land until
     * the sim owns the cell grid, so this records nothing today.
     */
    paint(_from: Vec2, _to: Vec2, _straight: boolean): void {
      // Intentionally empty until the sim lands.
    },

    /**
     * SEAM — needs the sim.
     *
     * alt+click picks the material under the cursor, which means sampling a cell the
     * client does not own yet.
     */
    pickMaterialAt(_world: Vec2): void {
      // Intentionally empty until the sim lands.
    },
  };
}

export type Actions = ReturnType<typeof createActions>;
