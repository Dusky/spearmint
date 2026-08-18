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
}

interface RawElement {
  id: number;
  name: string;
  state: string;
  color: string;
}

export const ELEMENTS: readonly ElementInfo[] = (elementData.elements as RawElement[]).map(
  (element) => ({
    id: element.id,
    name: element.name,
    state: element.state as ElementInfo['state'],
    color: element.color,
  }),
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
