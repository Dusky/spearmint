import elementData from '../../data/elements.json';

/**
 * Element definitions, read from the same file the simulation compiles in.
 *
 * One source of truth rather than two: ids are declared in the data (spec 3.2) and are
 * permanent, so the client must never assume them. Importing the file means a new
 * element appears in the UI the moment it exists in the sim.
 */
export interface ElementInfo {
  readonly id: number;
  readonly name: string;
  readonly state: 'solid' | 'powder' | 'liquid' | 'gas';
  readonly color: string;
  /** Scenery, not a material anyone chooses — a belt's own structural fill, so far.
   *  Sim-core never reads this field (spec 3.2 tolerates unknown fields); it exists so
   *  the client can hide such rows from every player-facing element picker. */
  readonly internal: boolean;
  /** Kelvin at which this turns into `meltsInto`. Meaningless when nothing does. */
  readonly meltingPoint: number;
  /** What this becomes once it reaches `meltingPoint`, or null if nothing does — which
   *  is still true of most of the roster. */
  readonly meltsInto: string | null;
}

interface RawElement {
  id: number;
  name: string;
  state: string;
  color: string;
  internal?: boolean;
  melting_point: number;
  meltsInto?: string;
}

export const ELEMENTS: readonly ElementInfo[] = (elementData.elements as RawElement[]).map(
  (element) => ({
    id: element.id,
    name: element.name,
    state: element.state as ElementInfo['state'],
    color: element.color,
    internal: element.internal ?? false,
    meltingPoint: element.melting_point,
    meltsInto: element.meltsInto ?? null,
  }),
);

/**
 * The coldest temperature at which anything in the world turns into something else.
 *
 * Derived rather than written down, so it follows the data: the moment an element with
 * a lower melting point ships, the inspector starts measuring against that instead.
 * `Infinity` if nothing melts at all, which reads correctly as "no heat is ever enough".
 */
export const LOWEST_MELTING_POINT: number = Math.min(
  ...ELEMENTS.filter((element) => element.meltsInto !== null).map(
    (element) => element.meltingPoint,
  ),
);

/**
 * Everything a filter could plausibly be tuned to — the full roster, not the curated
 * three the draw tool offers. A filter only ever routes matter that already exists in
 * the world; unlike drawing or spawning it, letting a player pick gold or fuel here
 * cannot hand them anything for free.
 */
export const FILTER_TARGETS: readonly ElementInfo[] = ELEMENTS.filter(
  (element) => !element.internal,
);

export const EMPTY_ELEMENT = 0;

/**
 * Whether an emitter could emit this.
 *
 * Solids do not flow: an emitter would place one cell and stop. Drawing is the tool for
 * structure, and this keeps a wall emitter from being offered at all.
 */
export function canBeEmitted(element: ElementInfo): boolean {
  return element.state !== 'solid';
}

export function elementByName(name: string): ElementInfo {
  const found = ELEMENTS.find((element) => element.name === name);
  if (!found) throw new Error(`no element named ${name} in data/elements.json`);
  return found;
}
