/* SCAFFOLDING — delete when the sim and the economy server land.
 *
 * The HUD needs something to display. This file fabricates it: the initial state
 * reproduces the designed screen value-for-value, and `startPlaceholderFeed` nudges
 * the numbers so the chrome can be judged against live content.
 *
 * Nothing here is a reference implementation of anything. The real readouts come from
 * the Rust/WASM sim and the real economy is server-owned (spec §8.1). */

import { BASE_SPAWNERS, DEFAULT_ZOOM, TILE_CELLS } from '../constants';
import type { Blueprint, GameState, ParticleKind } from './types';
import { PARTICLE_KINDS } from './types';

const kindIndex = (kind: ParticleKind): number => PARTICLE_KINDS.indexOf(kind);

/** Paints a cell grid the way a player would have drawn one. */
function buildCells(
  cellWidth: number,
  cellHeight: number,
  paint: (rect: (x0: number, y0: number, x1: number, y1: number, kind: ParticleKind) => void) => void,
): Uint8Array {
  const cells = new Uint8Array(cellWidth * cellHeight);
  paint((x0, y0, x1, y1, kind) => {
    const value = kindIndex(kind);
    for (let y = Math.max(0, y0); y <= Math.min(cellHeight - 1, y1); y++) {
      for (let x = Math.max(0, x0); x <= Math.min(cellWidth - 1, x1); x++) {
        cells[y * cellWidth + x] = value;
      }
    }
  });
  return cells;
}

function washer(): Blueprint {
  const cellWidth = 24 * TILE_CELLS;
  const cellHeight = 20 * TILE_CELLS;
  return {
    id: 'washer-02',
    name: 'Washer 02',
    width: 24,
    height: 20,
    yieldBest: 0.82,
    cellWidth,
    cellHeight,
    cells: buildCells(cellWidth, cellHeight, (rect) => {
      rect(24, 24, 192, 32, 'structure');
      rect(24, 24, 32, 148, 'structure');
      rect(184, 24, 192, 148, 'structure');
      rect(32, 140, 184, 148, 'structure');
      rect(40, 60, 176, 138, 'sand');
      rect(40, 36, 176, 58, 'water');
    }),
  };
}

function settler(): Blueprint {
  const cellWidth = 18 * TILE_CELLS;
  const cellHeight = 26 * TILE_CELLS;
  return {
    id: 'settler-a',
    name: 'Settler A',
    width: 18,
    height: 26,
    yieldBest: 0.71,
    cellWidth,
    cellHeight,
    cells: buildCells(cellWidth, cellHeight, (rect) => {
      rect(16, 16, 146, 24, 'structure');
      rect(40, 24, 48, 208, 'structure');
      rect(114, 24, 122, 208, 'structure');
      rect(48, 120, 114, 208, 'water');
      rect(48, 190, 114, 208, 'wetSand');
    }),
  };
}

function slagDump(): Blueprint {
  const cellWidth = 40 * TILE_CELLS;
  const cellHeight = 12 * TILE_CELLS;
  return {
    id: 'slag-dump',
    name: 'Slag dump',
    width: 40,
    height: 12,
    // No output, so no yield was ever observed. Meta shows footprint only.
    yieldBest: null,
    cellWidth,
    cellHeight,
    cells: buildCells(cellWidth, cellHeight, (rect) => {
      rect(16, 96, 344, 104, 'structure');
      rect(16, 24, 344, 30, 'structure');
      rect(24, 40, 336, 96, 'wetSand');
    }),
  };
}

/** The playable arena, in cells. Must match the size `main.ts` builds the world at. */
export const ARENA_WIDTH = 420;
export const ARENA_HEIGHT = 260;

export function initialState(): GameState {
  return {
    ui: {
      selectedTool: 'draw',
      selectedMaterial: 'structure',
      // Gold is the flagship case (spec 4.3): "run the wash output along a filter set
      // to gold and the nuggets drop into the vault while the rest carries on."
      filterTarget: 'gold',
      selection: null,
      showTileGrid: false,
      // Centred on the arena. Starting outside it would show empty space with no floor,
      // and anything drawn there falls forever (spec 3.6).
      camera: {
        x: ARENA_WIDTH / 2,
        y: ARENA_HEIGHT / 2,
        zoom: DEFAULT_ZOOM,
      },
      cursor: { x: ARENA_WIDTH / 2, y: ARENA_HEIGHT / 2 },
      strokeAnchor: null,
      painting: false,
      openDrawer: null,
    },
    // Gold, the rate and the spawner count are all measured from the simulation once
    // it starts reporting. `spawnersMax` is the game's hard input cap (spec 3.4) and is
    // the one number here the economy server will eventually own.
    economy: {
      gold: 0,
      goldRate: 0,
      spawnersOwned: 0,
      pressesOwned: 0,
      vaultsOwned: 0,
      spawnersMax: BASE_SPAWNERS,
      blueprintSlots: 4,
      purchasedUpgrades: [],
    },
    /* Spawner capacity is bought from the capability drawer and priced from the cap
     * itself, so it needs no row here. The rest of the upgrade list was placeholder
     * copy for things that do not exist; a drawer showing one real purchase beats one
     * showing three imaginary ones. */
    upgrades: [],
    // PLACEHOLDER: blueprints are out of the slice entirely.
    blueprints: [washer(), settler(), slagDump()],
    // The marquee and the problem notice hide themselves until there is something real
    // to show. The readout arrives with the first measurement from the simulation.
    constructs: [],
    readout: null,
    notices: [],
    tick: 1284905,
    /* Halved from 60 after playtesting: the whole world read as too fast to watch, let
     * alone build in. The fixed timestep is the host's (spec 3.1), so this is a pure
     * presentation decision the simulation core knows nothing about — it cannot affect
     * determinism. Per-machine pacing is separate, and lives in `data/entities.json`
     * as each entity's `interval`. */
    tickRate: 30,
    seed: 0x4f2a11,
  };
}

