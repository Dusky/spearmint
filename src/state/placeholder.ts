/* SCAFFOLDING — delete when the sim and the economy server land.
 *
 * The HUD needs something to display. This file fabricates it: the initial state
 * reproduces the designed screen value-for-value, and `startPlaceholderFeed` nudges
 * the numbers so the chrome can be judged against live content.
 *
 * Nothing here is a reference implementation of anything. The real readouts come from
 * the Rust/WASM sim and the real economy is server-owned (spec §8.1). */

import { DEFAULT_ZOOM, READOUT_HZ, TILE_CELLS } from '../constants';
import type { Store } from './store';
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
      rect(24, 24, 192, 32, 'wall');
      rect(24, 24, 32, 148, 'wall');
      rect(184, 24, 192, 148, 'wall');
      rect(32, 140, 184, 148, 'wall');
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
      rect(16, 16, 146, 24, 'wall');
      rect(40, 24, 48, 208, 'wall');
      rect(114, 24, 122, 208, 'wall');
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
      rect(16, 96, 344, 104, 'wall');
      rect(16, 24, 344, 30, 'wall');
      rect(24, 40, 336, 96, 'wetSand');
    }),
  };
}

/** Tile-space bounds of the machine the player has selected. */
const WASHER_BOUNDS = { x: 140, y: -48, width: 24, height: 20 } as const;

export function initialState(): GameState {
  return {
    ui: {
      selectedTool: 'draw',
      selectedMaterial: 'wall',
      selection: 'washer-02-instance',
      showTileGrid: false,
      camera: {
        x: (WASHER_BOUNDS.x + WASHER_BOUNDS.width / 2) * TILE_CELLS,
        y: (WASHER_BOUNDS.y + WASHER_BOUNDS.height / 2) * TILE_CELLS,
        zoom: DEFAULT_ZOOM,
      },
      cursor: { x: 1284, y: -406 },
      openDrawer: null,
    },
    economy: {
      gold: 4812,
      goldRate: 38,
      spawnersOwned: 3,
      spawnersMax: 5,
      blueprintSlots: 4,
      purchasedUpgrades: [],
    },
    upgrades: [
      {
        id: 'teleport-range-5',
        title: 'Teleport range 4 → 5 cells',
        description: 'Straight line only.',
        price: 2400,
      },
      {
        id: 'blueprint-slot-4',
        title: 'Blueprint slot 3 → 4',
        // NOTE: this copy answers spec open question 5 (paste fidelity) in favour of
        // "physics actually matters". Flagged, not settled — see README.
        description: 'Pasted machines obey local conditions.',
        price: 1150,
      },
      {
        id: 'spawner-4',
        title: 'Spawner 4',
        description: 'The only thing that raises your ceiling.',
        price: 18000,
      },
    ],
    blueprints: [washer(), settler(), slagDump()],
    constructs: [
      {
        id: 'washer-02-instance',
        name: 'Washer 02',
        bounds: WASHER_BOUNDS,
        running: true,
        blueprintId: 'washer-02',
      },
    ],
    readout: {
      constructId: 'washer-02-instance',
      yieldCurrent: 0.68,
      temperature: 384,
      contactArea: 1940,
      residence: 41,
      mixing: 0.62,
    },
    notices: [
      {
        id: 'belt-07-buried',
        message: 'Belt 07 is buried — slag backed up 340 cells',
        at: { x: 2260, y: -180 },
      },
    ],
    tick: 1284905,
    tickRate: 60,
    seed: '4f2a11',
  };
}

/**
 * Drives the placeholder numbers. Ticks accumulate on a fixed timestep — sim ticks
 * are decoupled from render frames (spec §3.1) — while the store is only written at
 * READOUT_HZ, so the DOM sees a readable cadence rather than one write per frame.
 *
 * Returns a stop function.
 */
export function startPlaceholderFeed(store: Store<GameState>): () => void {
  const tickMs = 1000 / store.state.tickRate;
  const readoutMs = 1000 / READOUT_HZ;

  let last = performance.now();
  let tickDebt = 0;
  let sinceReadout = 0;
  let pendingTicks = 0;
  let phase = 0;
  let frame = 0;

  const loop = (now: number): void => {
    frame = requestAnimationFrame(loop);
    const elapsed = Math.min(now - last, 250); // a backgrounded tab must not surge
    last = now;

    tickDebt += elapsed;
    const ticks = Math.floor(tickDebt / tickMs);
    tickDebt -= ticks * tickMs;
    pendingTicks += ticks;

    sinceReadout += elapsed;
    if (sinceReadout < readoutMs) return;
    const seconds = sinceReadout / 1000;
    sinceReadout = 0;
    phase += seconds;

    const ticked = pendingTicks;
    pendingTicks = 0;

    store.update((state) => ({
      ...state,
      tick: state.tick + ticked,
      economy: { ...state.economy, gold: state.economy.gold + state.economy.goldRate * seconds },
      readout: state.readout && {
        ...state.readout,
        // Gentle drift only. Residence stays short so the designed diagnosis holds.
        yieldCurrent: 0.68 + 0.03 * Math.sin(phase * 0.7),
        temperature: 384 + 6 * Math.sin(phase * 0.5),
        contactArea: 1940 + 90 * Math.sin(phase * 0.31),
        residence: 41 + 3 * Math.sin(phase * 0.9),
        mixing: 0.62 + 0.04 * Math.sin(phase * 0.43),
      },
    }));
  };

  frame = requestAnimationFrame(loop);
  return () => cancelAnimationFrame(frame);
}
