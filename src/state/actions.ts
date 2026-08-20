/** Every state change the HUD can make. Keeping them here means the input layer and
 *  the components share one vocabulary, and the shape of what the sim and the server
 *  will eventually own stays visible in one file. */

import { spawnerSlotPrice, SPEEDS, TILE_CELLS } from '../constants';
import { constrainToAxis, placementRefusal, strokeTiles, toTile } from './build';
import { ELEMENTS, elementByName, FILTER_TARGETS, EMPTY_ELEMENT } from '../sim/elements';
import { entityByKind, entityForTool, isToolBuilt } from '../sim/entities';
import { MATERIALS } from './types';
import type { Store } from './store';
import type {
  DrawerName,
  GameState,
  Material,
  NoticeId,
  SelectedMachine,
  Tool,
  Vec2,
} from './types';

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
  entityKind(index: number): number | null;
  entityEnabled(index: number): boolean | null;
  setEntityEnabled(index: number, enabled: boolean): void;
  entityRate(index: number): number | null;
  setEntityRate(index: number, rate: number): void;
  temperatureAt(x: number, y: number): number;
  setHeatOverlay(enabled: boolean): void;
}

/** Materials are element names, so these resolve straight out of the data file. */
const MATERIAL_ELEMENTS: Record<Material, number> = Object.fromEntries(
  MATERIALS.map((material) => [material, elementByName(material).id]),
) as Record<Material, number>;

const MATERIAL_BY_ELEMENT = new Map<number, Material>(
  MATERIALS.map((material) => [MATERIAL_ELEMENTS[material], material]),
);

/** A filter's target names are the full element roster, so this is a straight lookup
 *  rather than a curated table like `MATERIAL_ELEMENTS`. */
const FILTER_TARGET_NAMES = new Set(FILTER_TARGETS.map((element) => element.name));

export function createActions(store: Store<GameState>, sim: SimBridge) {
  const patchUi = (patch: Partial<GameState['ui']>): void => {
    store.update((state) => ({ ...state, ui: { ...state.ui, ...patch } }));
  };

  /** Reads a machine's live settings out of the sim, for the store to mirror. */
  const readMachine = (index: number | null): SelectedMachine | null => {
    if (index === null) return null;
    const kind = sim.entityKind(index);
    const enabled = sim.entityEnabled(index);
    const rate = sim.entityRate(index);
    if (kind === null || enabled === null || rate === null) return null;
    return { index, kind, enabled, rate };
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

    /** What a filter is tuned to. The full element roster, not the curated three
     *  `selectMaterial` offers — a filter only ever routes matter that already exists,
     *  so letting it target gold or fuel cannot hand the player anything for free. */
    selectFilterTarget(elementName: string): void {
      if (!FILTER_TARGET_NAMES.has(elementName)) return;
      patchUi({ filterTarget: elementName });
    },

    moveCursor(cursor: Vec2): void {
      const current = store.state.ui.cursor;
      if (current.x === cursor.x && current.y === cursor.y) return;
      // Described here rather than in the readout feed: that runs at a few hertz, which
      // is fine for numbers and far too slow for a label chasing a pointer.
      patchUi({ cursor, hoverLabel: this.describeAt(cursor) });
    },

    /** What is under a world cell, in words, or null if tooltips are off or it is
     *  empty space. A machine wins over the cell it is drawn on: you hover a press to
     *  ask about the press, not about the sand falling through it. */
    describeAt(world: Vec2): string | null {
      if (!store.state.ui.showTooltips) return null;

      // Reported always, not only when it differs from ambient. This is a probe: "it is
      // still stone cold" is exactly the reading you are hovering to get, and a number
      // that only sometimes appears is one you cannot trust the absence of.
      const temperature = `${sim.temperatureAt(world.x, world.y)}K`;

      const index = sim.entityAt(world.x, world.y);
      if (index !== null) {
        const kind = sim.entityKind(index);
        const machine = kind === null ? undefined : entityByKind(kind);
        if (machine) {
          const off = sim.entityEnabled(index) === false ? ' · off' : '';
          return `${machine.name}${off} · ${temperature}`;
        }
      }

      const element = sim.elementAt(world.x, world.y);
      const name = ELEMENTS.find((candidate) => candidate.id === element)?.name;
      // The void reads as nothing rather than as a cold nothing — there is no matter
      // there to have a temperature, only a grid cell that records one.
      if (name === undefined) return null;
      return `${name} · ${temperature}`;
    },

    toggleTooltips(): void {
      const showTooltips = !store.state.ui.showTooltips;
      // Patched first, because `describeAt` reads the flag and would otherwise still be
      // answering for the old one. Recomputed rather than left null so switching them
      // on describes what is already under the cursor, instead of staying blank until
      // the pointer happens to cross into another cell.
      patchUi({ showTooltips });
      patchUi({ hoverLabel: this.describeAt(store.state.ui.cursor) });
    },

    /** Tints cells by temperature. Heat is otherwise invisible: it moves through
     *  matter that looks identical either way, so a cold factory and a factory whose
     *  heat is not reaching anything are the same picture. */
    toggleHeatOverlay(): void {
      const showHeat = !store.state.ui.showHeat;
      sim.setHeatOverlay(showHeat);
      patchUi({ showHeat });
    },

    /** How fast the sim runs, as a multiple of its tick rate. Zero pauses it.
     *
     *  Only the host's timestep changes (spec 3.1) — the sim still advances in whole
     *  ticks and is simply asked for fewer or more of them, so nothing here can reach
     *  determinism. */
    setSpeed(speed: number): void {
      if (!SPEEDS.includes(speed as (typeof SPEEDS)[number])) return;
      patchUi({ speed });
    },

    /** Space is taken by panning, so pause is its own key. Resumes to full speed rather
     *  than to whatever it was: a remembered 0.25x that only reappears on unpause is a
     *  worse surprise than losing the setting. */
    togglePause(): void {
      this.setSpeed(store.state.ui.speed === 0 ? 1 : 0);
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
        case 'select':
          this.selectEntityAt(world);
          return;
        case 'spawner':
        case 'press':
        case 'compactor':
        case 'heater':
          this.placeMachine(world);
          return;
        case 'vault':
        case 'belt':
        case 'filter':
        case 'kiln':
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
        element = elementByName(ui.filterTarget).id;
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
      // Removing shifts every later index, so a held selection would silently start
      // pointing at a different machine.
      patchUi({ selectedEntity: null });
      this.countMachines();
      return true;
    },

    /**
     * Picks the machine under the cursor, or clears the selection when there is none.
     *
     * This is what the properties panel reads. It is a tool of its own rather than a
     * modifier on the others because every other tool's click already means "place
     * this here", and overloading that would make selecting a machine and building
     * next to one the same gesture.
     */
    selectEntityAt(world: Vec2): void {
      patchUi({ selectedEntity: readMachine(sim.entityAt(world.x, world.y)) });
    },

    /** Switches the selected machine on or off, for working on a running factory. */
    setSelectedEnabled(enabled: boolean): void {
      const selected = store.state.ui.selectedEntity;
      if (!selected) return;
      sim.setEntityEnabled(selected.index, enabled);
      patchUi({ selectedEntity: readMachine(selected.index) });
    },

    /** Retunes how much the selected machine does per action. */
    setSelectedRate(rate: number): void {
      const selected = store.state.ui.selectedEntity;
      if (!selected) return;
      sim.setEntityRate(selected.index, Math.max(1, Math.round(rate)));
      patchUi({ selectedEntity: readMachine(selected.index) });
    },

    /** Removes the selected machine, and clears the selection with it. */
    removeSelected(): void {
      const selected = store.state.ui.selectedEntity;
      if (!selected) return;
      sim.removeEntity(selected.index);
      patchUi({ selectedEntity: null });
      this.countMachines();
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
