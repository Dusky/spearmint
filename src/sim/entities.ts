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
}

interface RawEntity {
  id: number;
  name: string;
  tool: string;
  widthTiles: number;
  heightTiles: number;
}

export const ENTITIES: readonly EntityInfo[] = (entityData.entities as RawEntity[]).map(
  (entity) => ({
    id: entity.id,
    name: entity.name,
    tool: entity.tool,
    widthTiles: entity.widthTiles,
    heightTiles: entity.heightTiles,
  }),
);

/** The machine a tool places, or undefined for a tool that places none. */
export function entityForTool(tool: string): EntityInfo | undefined {
  return ENTITIES.find((entity) => entity.tool === tool);
}

/** Tools that do something without placing a machine. Everything else has to exist in
 *  the data to be real. */
const TOOLS_WITHOUT_MACHINES = ['draw', 'erase'];

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
