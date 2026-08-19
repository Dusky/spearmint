import entityData from '../../data/entities.json';

/**
 * Machine definitions, read from the same file the simulation compiles in.
 *
 * One source of truth. Kind ids are declared in the data and are permanent, so the
 * client must never assume them — and a new machine appears in the tool column the
 * moment it exists in the data.
 */
export interface EntityInfo {
  readonly id: number;
  readonly name: string;
  /** Which tool places it, matching a name in `TOOLS`. */
  readonly tool: string;
  readonly widthTiles: number;
  readonly heightTiles: number;
  /** What it does. The properties panel keys off this to decide which controls a
   *  given machine even has — a belt has no rate to set, a vault has nothing at all. */
  readonly behaviour: string;
  /** How much it does per action, as its type declares. A placed machine can override
   *  this per instance; this is the default the panel shows the slider around. */
  readonly rate: number;
}

interface RawEntity {
  id: number;
  name: string;
  tool: string;
  widthTiles: number;
  heightTiles: number;
  behaviour: string;
  rate?: number;
}

export const ENTITIES: readonly EntityInfo[] = (entityData.entities as RawEntity[]).map(
  (entity) => ({
    id: entity.id,
    name: entity.name,
    tool: entity.tool,
    widthTiles: entity.widthTiles,
    heightTiles: entity.heightTiles,
    behaviour: entity.behaviour,
    rate: entity.rate ?? 1,
  }),
);

/** The machine a tool places, or undefined for a tool that places none. */
export function entityForTool(tool: string): EntityInfo | undefined {
  return ENTITIES.find((entity) => entity.tool === tool);
}

/** The machine of a given kind id, as `sim.entityKind` reports it. */
export function entityByKind(kind: number): EntityInfo | undefined {
  return ENTITIES.find((entity) => entity.id === kind);
}

/** Whether this machine's throughput is a thing worth offering a control for.
 *
 *  A belt or a vault has a `rate` in the data only because the field defaults; nothing
 *  reads it, so a slider for it would be a lie. */
export function hasRate(entity: EntityInfo): boolean {
  return ['emit', 'press', 'refine', 'heater'].includes(entity.behaviour);
}

/** Tools that do something without placing a machine. Everything else has to exist in
 *  the data to be real. */
const TOOLS_WITHOUT_MACHINES = ['select', 'draw', 'erase'];

/**
 * Whether this tool actually does anything yet.
 *
 * Derived rather than listed: a tool is built when it draws, erases, or resolves to a
 * machine in `data/entities.json`. Adding a belt there makes the belt tool live with no
 * edit here — and until then the tool column can say so rather than pretending.
 */
export function isToolBuilt(tool: string): boolean {
  return TOOLS_WITHOUT_MACHINES.includes(tool) || entityForTool(tool) !== undefined;
}
