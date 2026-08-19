/* The UI state model, transcribed from the handoff's "State" section.
 *
 * Two rules from the spec shape these types:
 *  - Economy is server-authoritative (§8.1) and must stay serializable and
 *    server-shaped even while v1 keeps it locally (§8.5).
 *  - Nothing here holds a throughput number. Particles are never abstracted
 *    (§1.1) and there is no offline accrual to feed (§6), so the interface
 *    reports physical conditions and never a rate of production. */

/* Ordered so the column can be grouped (see TOOL_GROUPS): select and the drawing tools
 * first, then transport, then the machines in the order material actually moves —
 * input, convert to currency, burn the byproduct, compact what's burnt, heat — then
 * the places you put things. Hotkeys are row order, so this is also the key order. */
export const TOOLS = [
  'select',
  'draw',
  'erase',
  'belt',
  'filter',
  'spawner',
  'press',
  'burner',
  'compactor',
  'heater',
  'vault',
  'teleport',
  'blueprint',
] as const;
export type Tool = (typeof TOOLS)[number];

/** The tool column, in labelled groups. A flat list of thirteen is a wall of text to
 *  scan; these are the four questions you are actually asking ("what am I drawing with",
 *  "how does material move", "what processes it", "where does it go"). */
export const TOOL_GROUPS: readonly { readonly label: string; readonly tools: readonly Tool[] }[] = [
  { label: 'BUILD', tools: ['select', 'draw', 'erase'] },
  { label: 'TRANSPORT', tools: ['belt', 'filter', 'teleport'] },
  { label: 'MACHINES', tools: ['spawner', 'press', 'burner', 'compactor', 'heater'] },
  { label: 'PLACES', tools: ['vault', 'blueprint'] },
];

/**
 * What the draw tool can place, by element name from `data/elements.json`.
 *
 * The design handoff shows four swatches — structure, insulator, heater, cooler. Only
 * three of those elements exist, and inventing swatches for the other two would put
 * controls on screen that do nothing. This list grows when the roster does (spec 11 q7).
 */
export const MATERIALS = ['structure', 'sand', 'water'] as const;
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
export const PARTICLE_KINDS = ['empty', 'structure', 'sand', 'water', 'wetSand'] as const;
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

/** A placed machine as the properties panel sees it: which one, and the settings a
 *  player can change without removing and replacing it. */
export interface SelectedMachine {
  /** Its index in the sim. Shifts when an earlier machine is removed, which is why
   *  removing anything clears the selection rather than trying to track it. */
  readonly index: number;
  /** Its kind id, for looking the machine up in `data/entities.json`. */
  readonly kind: number;
  readonly enabled: boolean;
  /** How much it does per action. */
  readonly rate: number;
}

export interface UiState {
  readonly selectedTool: Tool;
  readonly selectedMaterial: Material;
  /** What a filter is tuned to, by element name — the full roster (`FILTER_TARGETS`),
   *  not the curated three `selectedMaterial` offers, since a filter only routes
   *  matter that already exists rather than creating any. */
  readonly filterTarget: string;
  /** The selected construct's id, or null. Drives the inspector and the marquee. */
  readonly selection: ConstructId | null;
  /** The selected machine, or null. What the properties panel shows and retunes.
   *
   *  Separate from `selection` above, which addresses the placeholder `constructs`
   *  list rather than anything the simulation knows about. A mirror rather than just
   *  an index: components are handed state and nothing else, so the values they render
   *  have to be in it. The sim stays authoritative — this is refreshed from it on every
   *  selection and every retune. */
  readonly selectedEntity: SelectedMachine | null;
  /** Whether hovering the world describes what is under the cursor. */
  readonly showTooltips: boolean;
  /** What is under the cursor, when tooltips are on. Computed as the cursor moves —
   *  the readout feed runs at a few hertz, which is too slow for something that has to
   *  track a pointer. Null when there is nothing there or tooltips are off. */
  readonly hoverLabel: string | null;
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
