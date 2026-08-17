/** A minimal observable store. State is replaced, never mutated, so components can
 *  compare slices by identity and skip work.
 *
 *  Deliberately not a framework: the HUD is a fixed set of components that build their
 *  DOM once and write into it. Nothing here is diffed or re-created per frame. */
export type Listener<T> = (state: T, previous: T) => void;

export class Store<T> {
  #state: T;
  readonly #listeners = new Set<Listener<T>>();

  constructor(initial: T) {
    this.#state = initial;
  }

  get state(): T {
    return this.#state;
  }

  /** Applies `next`. Returning the same object is a no-op and notifies nobody. */
  update(next: (state: T) => T): void {
    const previous = this.#state;
    const updated = next(previous);
    if (updated === previous) return;
    this.#state = updated;
    for (const listener of this.#listeners) listener(updated, previous);
  }

  subscribe(listener: Listener<T>): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }
}
