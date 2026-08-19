/// <reference types="vite/client" />

/** The exports the wasm module actually provides. Kept in one place so the shape of
 *  the boundary is visible; see `crates/sim-wasm/src/lib.rs` for the other side. */
interface SimExports {
  readonly memory: WebAssembly.Memory;
  sim_init(seed: number, width: number, height: number): number;
  sim_step(ticks: number): void;
  sim_tick(): number;
  sim_render(originX: number, originY: number, width: number, height: number): number;
  sim_paint_tiles(
    tileX0: number,
    tileY0: number,
    tileX1: number,
    tileY1: number,
    id: number,
  ): void;
  sim_get(x: number, y: number): number;
  sim_count(id: number): number;
  sim_collected(): number;
  sim_stored(): number;
  sim_spend(amount: number): number;
  sim_chunk_count(): number;
  sim_awake_chunk_count(): number;
  sim_set_sleeping(enabled: number): void;
  sim_place_entity(
    kind: number,
    tileX: number,
    tileY: number,
    width: number,
    height: number,
    element: number,
    direction: number,
  ): number;
  sim_entity_at(x: number, y: number): number;
  sim_remove_entity(index: number): number;
  sim_entity_count(): number;
  sim_entity_count_of_kind(kind: number): number;
  sim_entity_tile_x(index: number): number;
  sim_entity_tile_y(index: number): number;
  sim_contact_area(): number;
}

/**
 * The running simulation.
 *
 * This is the real Rust core compiled to wasm — not a reimplementation. A TypeScript
 * copy of the physics would duplicate the rules, discard the determinism work, and
 * simulate something the shipped game does not.
 */
export class Sim {
  readonly width: number;
  readonly height: number;
  readonly #exports: SimExports;

  private constructor(exports: SimExports, width: number, height: number) {
    this.#exports = exports;
    this.width = width;
    this.height = height;
  }

  static async load(seed: number, width: number, height: number): Promise<Sim> {
    // No bindgen, so no glue module to import — just the raw bytes.
    const { instance } = await WebAssembly.instantiateStreaming(fetch('/sim.wasm'), {});
    const exports = instance.exports as unknown as SimExports;

    if (exports.sim_init(seed, width, height) !== 1) {
      throw new Error('sim_init failed — element data did not load');
    }
    return new Sim(exports, width, height);
  }

  step(ticks: number): void {
    if (ticks > 0) this.#exports.sim_step(ticks);
  }

  get tick(): number {
    return this.#exports.sim_tick();
  }

  /**
   * Renders a window of the world, returning a view straight onto wasm memory — no
   * copy. The view is only valid until the next call into the module, since the
   * buffer is reused and growing it can move it.
   */
  render(
    originX: number,
    originY: number,
    width: number,
    height: number,
  ): Uint8ClampedArray<ArrayBuffer> {
    const pointer = this.#exports.sim_render(originX, originY, width, height);
    // `memory.buffer` is typed as ArrayBufferLike because a module *can* be built with
    // shared memory. This one is not, so it is a plain ArrayBuffer — which ImageData
    // requires.
    const buffer = this.#exports.memory.buffer as ArrayBuffer;
    return new Uint8ClampedArray(buffer, pointer, width * height * 4);
  }

  /** Paints a stroke of whole tiles. Building is grid-aligned (spec 2.3). */
  paintTiles(from: { x: number; y: number }, to: { x: number; y: number }, element: number): void {
    this.#exports.sim_paint_tiles(from.x, from.y, to.x, to.y, element);
  }

  elementAt(x: number, y: number): number {
    return this.#exports.sim_get(x, y);
  }

  count(element: number): number {
    return this.#exports.sim_count(element);
  }

  get chunkCount(): number {
    return this.#exports.sim_chunk_count();
  }

  get awakeChunkCount(): number {
    return this.#exports.sim_awake_chunk_count();
  }

  setSleeping(enabled: boolean): void {
    this.#exports.sim_set_sleeping(enabled ? 1 : 0);
  }

  /** Places a machine on a tile. False if something is already there. */
  /** Places an entity over a region of tiles. A width or height of 0 means "as the
   *  type says", which is what a click on a machine sends. `direction` only means
   *  anything to a belt or filter. */
  placeEntity(
    kind: number,
    tileX: number,
    tileY: number,
    element: number,
    width = 0,
    height = 0,
    direction = 1,
  ): boolean {
    return (
      this.#exports.sim_place_entity(kind, tileX, tileY, width, height, element, direction) === 1
    );
  }

  /** The machine covering a world cell, or null. */
  entityAt(x: number, y: number): number | null {
    const index = this.#exports.sim_entity_at(x, y);
    return index < 0 ? null : index;
  }

  /** Removes a machine and frees its slot in full. */
  removeEntity(index: number): boolean {
    return this.#exports.sim_remove_entity(index) === 1;
  }

  countOfKind(kind: number): number {
    return this.#exports.sim_entity_count_of_kind(kind);
  }

  /** Where a machine sits, in tiles, for outlining it. */
  entityTile(index: number): { x: number; y: number } | null {
    const x = this.#exports.sim_entity_tile_x(index);
    return x === -2147483648 ? null : { x, y: this.#exports.sim_entity_tile_y(index) };
  }

  /** Nuggets ever minted. Income, not balance. */
  get collected(): number {
    return this.#exports.sim_collected();
  }

  /** The balance: currency sitting inside a machine. Gold on the floor is matter, not
   *  money (spec 5.1). */
  get stored(): number {
    return this.#exports.sim_stored();
  }

  /** Takes nuggets out of the machines holding them. Returns how many it took, which is
   *  either all of them or none. */
  spend(amount: number): number {
    return this.#exports.sim_spend(amount);
  }

  /** Cells where two reactants are touching. Walks the world, so read it at the
   *  readout rate rather than per frame. */
  get contactArea(): number {
    return this.#exports.sim_contact_area();
  }
}
