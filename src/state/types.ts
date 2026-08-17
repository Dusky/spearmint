/* The UI state model, transcribed from the handoff's "State" section.
 *
 * Two rules from the spec shape these types:
 *  - Economy is server-authoritative (§8.1) and must stay serializable and
 *    server-shaped even while v1 keeps it locally (§8.5).
 *  - Nothing here holds a throughput number. Particles are never abstracted
 *    (§1.1) and there is no offline accrual to feed (§6), so the interface
 *    reports physical conditions and never a rate of production. */

export const TOOLS = ['draw', 'erase', 'belt', 'spawner', 'teleport', 'blueprint'] as const;
export type Tool = (typeof TOOLS)[number];

export const MATERIALS = ['wall', 'insulator', 'heater', 'cooler'] as const;
export type Material = (typeof MATERIALS)[number];

export type ConstructId = string;
export type BlueprintId = string;
export type NoticeId = string;
export type UpgradeId = string;

/** A point in world space, in particle cells. */
export interface Vec2 {
  readonly x: number;
  readonly y: number;
}

/** An axis-aligned box in tiles. */
export interface TileRect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** Server-owned. Everything here is progression state, not physics. */
export interface EconomyState {
  readonly gold: number;
  /** Gold per second, as observed by the client. Display only. */
  readonly goldRate: number;
  readonly spawnersOwned: number;
  readonly spawnersMax: number;
  readonly blueprintSlots: number;
  readonly purchasedUpgrades: readonly UpgradeId[];
}

export interface Upgrade {
  readonly id: UpgradeId;
  readonly title: string;
  readonly description: string;
  readonly price: number;
}

export interface Blueprint {
  readonly id: BlueprintId;
  readonly name: string;
  /** Footprint in tiles. */
  readonly width: number;
  readonly height: number;
  /**
   * Best yield ever observed for this blueprint, 0–1, or null for a construct with
   * no output. A per-blueprint high-water mark, so it is stored with the blueprint
   * and not with any one machine instance.
   */
  readonly yieldBest: number | null;
  /** Cell-space extent of `cells`. */
  readonly cellWidth: number;
  readonly cellHeight: number;
  /**
   * The blueprint's actual construct data: one byte per cell, row-major, indexing
   * PARTICLE_KINDS. Thumbnails are rendered from this at the sim's own palette —
   * they are the real thing, not an icon.
   *
   * A typed array rather than a cell list because that is the shape the sim hands
   * across the WASM boundary (spec §9), so the thumbnail renderer will not need
   * rewriting when real blueprint data arrives.
   */
  readonly cells: Uint8Array;
}

/** Index 0 is empty; the rest are the sim's own palette entries. */
export const PARTICLE_KINDS = ['empty', 'wall', 'sand', 'water', 'wetSand'] as const;
export type ParticleKind = (typeof PARTICLE_KINDS)[number];

/** A placed machine. Instances carry live measurements; blueprints carry the record. */
export interface Construct {
  readonly id: ConstructId;
  readonly name: string;
  readonly bounds: TileRect;
  readonly running: boolean;
  /** The blueprint this was pasted from, if any — where `yieldBest` lives. */
  readonly blueprintId: BlueprintId | null;
}

/** Read-only, sim-derived, per selection. Physical conditions only. */
export interface SimReadout {
  readonly constructId: ConstructId;
  /** Current yield, 0–1. */
  readonly yieldCurrent: number;
  /** Kelvin. */
  readonly temperature: number;
  /** Particle cells in contact between reactants. */
  readonly contactArea: number;
  /** Ticks a particle spends inside the machine. */
  readonly residence: number;
  /** 0–1. */
  readonly mixing: number;
}

/** An active clog or burial. Emergent physics, not a rule — so this is information. */
export interface Notice {
  readonly id: NoticeId;
  readonly message: string;
  /** Where `F` jumps the camera to. */
  readonly at: Vec2;
}

export interface Camera {
  /** World position, in particle cells, of the viewport centre. */
  readonly x: number;
  readonly y: number;
  /** Rendered pixels per particle cell. */
  readonly zoom: number;
}

export type DrawerName = 'capability' | 'blueprints';

export interface UiState {
  readonly selectedTool: Tool;
  readonly selectedMaterial: Material;
  /** The selected construct's id, or null. Drives the inspector and the marquee. */
  readonly selection: ConstructId | null;
  readonly showTileGrid: boolean;
  readonly camera: Camera;
  /** Last known cursor position in world cells. Retained when the pointer leaves the
   *  viewport so the status bar readout does not blank out mid-drag. */
  readonly cursor: Vec2;
  /** Which overlay drawer is open. Only one at a time. */
  readonly openDrawer: DrawerName | null;
}

export interface GameState {
  readonly ui: UiState;
  readonly economy: EconomyState;
  readonly upgrades: readonly Upgrade[];
  readonly blueprints: readonly Blueprint[];
  readonly constructs: readonly Construct[];
  /** Live values for whichever construct is selected, or null. */
  readonly readout: SimReadout | null;
  /** Active notices. Only the most recent is shown. */
  readonly notices: readonly Notice[];
  /** Sim tick count. */
  readonly tick: number;
  /** Sim tick rate, Hz. */
  readonly tickRate: number;
  /** World seed, shown in the top bar. */
  readonly seed: string;
}
