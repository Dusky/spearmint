/* The UI state model, transcribed from the handoff's "State" section.
 *
 * Two rules from the spec shape these types:
 *  - Economy is server-authoritative (§8.1) and must stay serializable and
 *    server-shaped even while v1 keeps it locally (§8.5).
 *  - Nothing here holds a throughput number. Particles are never abstracted
 *    (§1.1) and there is no offline accrual to feed (§6), so the interface
 *    reports physical conditions and never a rate of production. */

/* Press, burner and compactor sit between spawner and vault in the order material
 * actually moves: input, convert to currency, burn the byproduct, compact what's
 * burnt, store. That pushes teleport and blueprint further down from the handoff's
 * 1-6, which is the lesser evil against live tools sitting below dead ones. */
export const TOOLS = [
  'draw',
  'erase',
  'belt',
  'spawner',
  'press',
  'burner',
  'compactor',
  'vault',
  'teleport',
  'blueprint',
] as const;
export type Tool = (typeof TOOLS)[number];

/**
 * What the draw tool can place, by element name from `data/elements.json`.
 *
 * The design handoff shows four swatches — wall, insulator, heater, cooler. Only three
 * elements exist, and inventing swatches for the other two would put controls on screen
 * that do nothing. This list grows when the element roster does (spec 11 q7).
 */
export const MATERIALS = ['wall', 'sand', 'water'] as const;
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
  /** The balance: nuggets sitting inside a machine. Measured from the world, not
   *  tallied — currency is matter (spec 5.1), so the pile is the number. */
  readonly gold: number;
  /** Gold per second, as observed by the client. Display only. */
  readonly goldRate: number;
  readonly spawnersOwned: number;
  readonly spawnersMax: number;
  /** Presses are uncapped — the cap that matters is on input (spec 3.4) — but the
   *  panel needs to know whether there is one at all to explain flat gold. */
  readonly pressesOwned: number;
  /** Same, for the vaults that make gold count as money. */
  readonly vaultsOwned: number;
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
// eslint-disable-next-line -- kept for the construct selection the design calls for
export interface Construct {
  readonly id: ConstructId;
  readonly name: string;
  readonly bounds: TileRect;
  readonly running: boolean;
  /** The blueprint this was pasted from, if any — where `yieldBest` lives. */
  readonly blueprintId: BlueprintId | null;
}

/**
 * Read-only, sim-derived. Every field is measured, not modelled.
 *
 * The design handoff's inspector shows temperature, residence and mixing alongside
 * these. Those are absent because nothing computes them: there is no heat system, and
 * residence needs a machine boundary that only per-construct selection would give.
 * Showing them would mean inventing numbers over real physics.
 *
 * Measurements are whole-world for now. Per-machine selection is the real design and
 * is what makes the inspector answer "why is *this* machine underperforming".
 */
export interface SimReadout {
  /** Fraction of the sand that has been washed, 0–1. */
  readonly yieldCurrent: number;
  /** Cells where two reactants are actually touching — where reactions can happen. */
  readonly contactArea: number;
  readonly sand: number;
  readonly water: number;
  readonly wetSand: number;
  /** Nuggets lying in the world with no machine holding them. Gold, but not money. */
  readonly looseGold: number;
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
  /** Where a shift-constrained stroke started, while one is in progress.
   *
   *  A constrained stroke previews rather than painting as it goes, so something has to
   *  hold the anchor for the ghost to draw from. Null the rest of the time. */
  readonly strokeAnchor: Vec2 | null;
  /** Suppresses the ghost while a free stroke is being painted — the paint is the
   *  feedback then, and a box under the cursor is just noise. */
  readonly painting: boolean;
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
  /** World seed. Held as a number because the simulation is seeded with it; the top
   *  bar renders it as hex. */
  readonly seed: number;
}
