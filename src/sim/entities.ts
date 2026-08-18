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
