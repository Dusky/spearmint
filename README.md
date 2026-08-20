# Particle Factory

An incremental factory game built on a side-on falling-sand particle simulation. See
[`docs/SPEC.md`](docs/SPEC.md) for the design and technical spec, and
[`design/particle-factory-ui/`](design/particle-factory-ui/) for the UI handoff this
client was built from.

The two halves are connected: the Rust simulation runs in the browser through wasm, and
you can draw a machine and watch material flow through it. See
[`docs/VERTICAL-SLICE.md`](docs/VERTICAL-SLICE.md) for what that slice is testing and
what is deliberately left out of it.

- **`crates/sim-core`** — the simulation. Headless, deterministic and proven so, on
  sparse chunks over an unbounded canvas, with viewport-gated sleeping. Twelve
  elements, eight machines, a conducting heat field and phase changes in both
  directions.
- **`crates/sim-wasm`** — the browser bridge. Raw `extern "C"` exports and a
  hand-written binding, so the core keeps its zero dependencies.
- **`crates/sim-harness`** — `sim-hash`, which runs the sim headless and prints world
  fingerprints. Also the shape the server-side replay verifier will take (§8.3).
- **`src/`** — the client: the HUD from the design handoff, drawing onto the real
  simulation.

What is real: the world, the physics, drawing, machines, belts, reactions, heat, and
every number in the top bar and the inspector. Gold is washed sand, the spawner count is
the sim's, and yield, contact area and temperature are measured from the world.

What is not real: purchasing. The capability and blueprint drawers (`C` and `B`) still
show placeholder rows, because the economy server and blueprints are both out of the
slice.

The inspector shows fewer measurements than the design handoff. Temperature arrived
with the heat system; residence and mixing are still missing because nothing computes
them — residence needs a machine boundary that per-construct selection would give.
Inventing numbers to fill the panel would be worse than a shorter one.

**Read [`docs/WHY-ITS-A-TOY.md`](docs/WHY-ITS-A-TOY.md) before adding anything.** The
slice has now been played, and the verdict was that the pieces do not compose into
decisions. That document has the mechanical reason and one proposal; the inventory it
ends with is the current state of the build.

## Running it

```sh
./setup.sh
```

Checks for Node and Rust, installs the wasm target and the npm packages, builds the
simulation, and starts the dev server on http://localhost:5173. `--setup-only` stops
before starting it; `--check` also runs the test suite. It tells you what to install if
something is missing rather than installing a toolchain behind your back.

Needs Node 20+ and Rust. On Arch: `sudo pacman -S nodejs npm rustup && rustup default
stable`.

Everything it does by hand:

```sh
npm install
npm run dev                 # builds the wasm module, then serves
npm run build               # wasm + typecheck + production build

cargo test --workspace      # or: npm run sim:test
cargo clippy --workspace --all-targets -- -D warnings
npm run sim:run -- --seed 0x4f2a11 --ticks 5000 --width 80 --height 50 --dump
```

The Rust toolchain is pinned in `rust-toolchain.toml`, which needs rustup to take
effect — the pin is for byte-identical reproducibility, so a system cargo runs the game
fine. Fonts are self-hosted, as the handoff requires; `node scripts/fetch-fonts.mjs`
regenerates `public/fonts/` and `src/styles/fonts.css`.

### What to try

Draw two sloping walls into a basin, switch to the spawner tool (`6`), put sand above
one side and water above the other, and watch the inspector. Yield rises when sand and
water actually touch, and the panel says so when they do not. That loop — draw, run,
measure, redraw — is the thing worth judging.

## The simulation core

Milestone 1 delivers exactly what spec §10 asks for: a fixed-point cell grid, a seeded
PRNG, a tick loop, three elements loaded from JSON, and a determinism test that holds
across runs, threads, and a process boundary.

Two decisions in it are worth knowing about, because both were the non-obvious option.

**Randomness is a pure function, not a stream.** The obvious implementation — one PRNG
advanced as cells are visited — is deterministic today and breaks the moment chunks
sleep (§2.4). A sleeping chunk draws no random numbers, so every cell after it would
get a *different* value than in a session where that chunk stayed awake, and the world
would diverge based on where the player was looking. Instead every draw is a hash of
`(seed, tick, x, y)`, which makes results independent of visit order, chunk boundaries,
sleeping, and any future threading — by construction rather than by discipline.

**`sim-core` has no dependencies, including for JSON.** Every JSON library parses
numbers as `f64`, so element data like `"thermal_conductivity": 0.27` would launder
itself through a float on the way into a sim that §3.1 forbids from containing one. The
crate carries its own JSON reader, which never interprets a number — it hands back the
raw text, and `Fixed::parse` converts it with integer arithmetic. That also keeps the
core free of the dependency tree and platform coupling §8.3 rules out.

### Chunking (Milestone 2a)

Storage is sparse chunks keyed by position — 16×16 tiles each, defined in tiles rather
than pixels so tile and chunk alignment cannot drift (§2.4). A chunk that never held
anything is not stored, which is what makes the unbounded canvas of §2.1 affordable.

**Chunking is storage and nothing else.** The tick still sweeps global rows bottom-up
across the whole live region, exactly as the flat grid does, so chunk layout cannot
influence the outcome. That is the point: it makes this step provably transparent, and
it quarantines the genuinely risky change — sleeping, where *what gets visited* stops
being a function of the world alone — into its own milestone.

The gate is `chunk_equivalence.rs`: a chunked world and a flat one must agree cell for
cell, across chunk seams, negative coordinates, and several seeds. Both run the *same*
tick loop, generic over the `CellField` trait — two implementations of the rules
agreeing would prove much less.

That equivalence turns out to rest on the Milestone 1 RNG decision. The chunked sweep
covers whole chunks, so it visits empty cells the flat sweep never sees; those cost
nothing and change nothing, because an empty cell is skipped without consuming any
randomness. With a streaming PRNG, every one of those extra visits would shift the
sequence and the two worlds would diverge on the first tick.

Sweeping whole chunks costs about 3× the flat world for the same scene. Dirty rects in
2b are the answer; the fix for the other half — a tree lookup per cell access — is
already in, as a one-entry chunk cache.

### Sleeping (Milestone 2b)

A chunk stops ticking when it is **quiescent** — nothing moved in it or in any of its
eight neighbours last tick. That is what makes sleeping free of consequence: ticking a
settled chunk produces no change, so skipping it produces no difference. The viewport
gate of §2.4 sits on top and can only keep *more* chunks awake, so it costs CPU and
cannot alter the world.

The wake rule is deliberately conservative — it can only over-simulate. Over-simulating
is invisible; under-simulating is a silent divergence.

The gate is `sleeping.rs`: a world where regions sleep must be identical to one where
nothing sleeps, across seeds, across chunk seams, with and without a viewport, and it
must conserve. A separate test asserts chunks *actually* sleep, because every other test
in the file would pass if nothing ever did.

This is also what separates sleeping from eviction. Sleeping preserves state exactly and
is unobservable; eviction discards it and is not, which is why it depends on an open
design question and this did not.

**It currently delivers no speedup, and that is a physics problem, not a scheduling
one.** Liquids never come to rest (§3.5): a void trapped in water random-walks forever,
because water never rises and a lateral move costs nothing. Any chunk holding water
stays dirty and keeps its neighbours awake. Powder and empty regions do sleep — a sand
heap reaches a genuine fixed point — so the machinery works and is waiting on a decision
about how liquids find their level.

### What "deterministic" is backed by

| Guarantee | How it is held |
|---|---|
| No floats | `#![deny(clippy::float_arithmetic)]` plus a test that reads the source. Both were mutation-checked: injecting a float fails each independently. |
| No random state | Randomness is a position hash; there is nothing to advance |
| No hash-map iteration | Element data lives in ordered `Vec`s; the JSON reader preserves member order |
| No system entropy or clock | Nothing in the crate can reach either |
| Nothing created or destroyed | Movement is always a swap, and a census test asserts it over 10,000 ticks (§1.1) |
| Rules never name an element | Physics dispatches on `state` — powder, liquid, solid — so adding an element is a data edit (§3.2) |

The world hash is FNV-1a, hand-rolled: `DefaultHasher` seeds itself from system entropy
per process and would fail the cross-process test for reasons unrelated to the sim. It
is canonical — keyed by absolute position, skipping empty cells — so a chunked world and
a flat one can be compared directly despite storing their cells nothing alike.

Debug and release builds produce identical hashes. A pinned golden hash makes any change
to the rules a decision rather than an accident, and it is pinned against the flat
world's original *structural* fingerprint: the move to chunked storage had to leave that
number untouched, which is how the refactor was shown to change no behaviour.

`sim-hash` is the harness binary. It exists for the cross-process test, for looking at a
headless world (`--dump` prints it as text), and as the shape the server-side replay
verifier of §8.3 will take.

## Controls

| Key | Action |
|---|---|
| `1`–`9` | Select tool, in tool-column order — `1` select, `2` draw, `3` erase, `4` belt, `5` filter, `6` spawner, `7` press, `8` burner, `9` compactor |
| `shift` | Constrain a stroke to a straight line |
| `space` + drag | Pan |
| `alt` + click | Pick the material under the cursor |
| click (spawner tool) | Place a spawner emitting the selected material |
| `F` | Jump to the current problem notice |
| `G` | Toggle the tile grid |
| `T` | Toggle hover tooltips |
| `H` | Toggle the heat overlay |
| `P` | Pause |
| `C` / `B` | Capability / Blueprints drawer |
| `Escape` | Close the drawer, then clear the selection |

The heater, vault, teleport and blueprint tools have no key — only single digits parse,
and the tool column runs to thirteen. Click them.

The tool keys, `shift`, `space`, `alt` and `F` are from the handoff. `G`, `T`, `H`, `P`,
`C`, `B` and `Escape` are not — see "Flagged back to design" below.

## Layout

```
data/elements.json          the element table — data, not code
crates/
  sim-core/                 the library. no I/O, no platform APIs, no dependencies
    src/fixed.rs            Q16.16 fixed-point, parsed without ever touching a float
    src/rng.rs              stateless position hash — the reason chunks can sleep later
    src/json.rs             minimal reader; numbers stay raw text
    src/elements.rs         element table + schema validation
    src/field.rs            CellField: the storage interface the rules are written to
    src/chunk.rs            sparse chunks over an unbounded canvas — the real storage
    src/grid.rs             one flat array with hard edges — the reference oracle
    src/step.rs             the tick rules, dispatched on state and never on identity
    src/world.rs            World (chunked, sleeps) and FlatWorld (reference)
    src/scene.rs            the starting world both backends are built from
    tests/                  determinism, chunk equivalence, conservation, no-floats
  sim-harness/              `sim-hash`: runs it headless, prints hashes, dumps worlds

src/
  constants.ts              tile size, default zoom, readout rate
  state/
    types.ts                the state model from the handoff's "State" section
    store.ts                observable store — state is replaced, never mutated
    actions.ts              every state change the HUD can make
    measurements.ts         inspector measurements + the diagnosis sentence
    placeholder.ts          SCAFFOLDING: fake state and a feed that nudges it
  sim/
    surface.ts              the seam between the HUD and whatever draws the world
    placeholderSurface.ts   SCAFFOLDING: a stand-in field, not a simulation
  ui/                       one file per component; each builds its DOM once
  styles/                   tokens, layout, generated fonts
```

The HUD is DOM overlaid on the sim canvas, as the handoff specifies — not an iframe,
not a canvas-drawn UI. Components build their DOM once and write into it on update;
nothing is diffed or re-created per frame, and there are no transitions anywhere.

### Where the real client plugs in

- **The renderer.** `SimSurface` (`src/sim/surface.ts`) is the only thing the HUD knows
  about the world. Swap `createPlaceholderSurface()` in `src/main.ts` for the WebGL2
  renderer over the Rust/WASM sim and no UI code changes.
- **Sim readouts.** `SimReadout` is populated by `startPlaceholderFeed` today. Point it
  at the sim instead. Values are written to the store at `READOUT_HZ` (4 Hz), not per
  frame — per-frame is unreadable and wasteful, and monospace keeps them from reflowing
  between writes.
- **The economy.** `EconomyState` is serializable and server-shaped already (spec §8.5).
  It needs an owner, not a redesign.
- **Two documented no-ops** in `src/state/actions.ts`, marked `SEAM`: `paint()` and
  `pickMaterialAt()`. Both interactions are settled in the design, but a stroke has
  nowhere to land and there is no cell to sample until the sim owns the grid. The input
  plumbing above them — tile snapping, `shift` constraint, drag tracking — is wired.

### The placeholder field

`placeholderSurface.ts` hashes world position into the sim's palette and pans that
with the camera, with a slow shimmer so the viewport is visibly alive. It is
deliberately **not** a cellular automaton: no state, no gravity, nothing to port. The
prototype's JS sand sim was not ported either — the handoff says not to, and the real
sim is Rust→WASM, fixed-point and deterministic (spec §3.1).

## Decisions worth knowing

- **The frame is fluid and edge-to-edge.** The prototype's 1160px width, outer border
  and 5px radius existed only because it sat on a page. Everything else is transcribed
  literally; where the handoff gives a pixel number, that number is in the CSS.
- **The two supporting panels are overlay drawers**, not a second row. The prototype
  laid them out flat so every state was visible at once.
- **"Rules of the chrome" is not implemented** — the handoff marks it documentation and
  says to delete it on implementation. Its content is the token table in
  `src/styles/tokens.css`.
- **The tool column is `content-box`.** The handoff's 150px is the column and the 1px
  border sits outside it; under `border-box` the four 26px swatches lose a pixel and
  wrap to a second row.
- **Capability rows are read-only.** Purchasing is server-owned (spec §8.1) and the
  transaction UI is undesigned, so the rows display and nothing more.
- **Blueprint thumbnails render from real cell data** at the sim's palette, via a
  `Uint8Array` per blueprint — the shape the sim will hand across the WASM boundary.
  The blueprint *data* is placeholder; the rendering path is not.

## Flagged back to design

Rather than inventing answers:

1. **The handoff answers spec open question 5.** The capability row "Blueprint slot
   3 → 4" reads *"Pasted machines obey local conditions"*, which picks the "physics
   actually matters" branch of §4.5. The spec lists paste fidelity as unresolved and
   says to pick one before implementing blueprints. Implemented as written; not treated
   as settled.
2. **Nothing opens the drawers.** The handoff says the supporting panels become
   overlays in-game but never says what summons them. `C` and `B` are placeholders, as
   is `G` for the tile grid (toggleable, off by default, no key given) and `Escape`.
3. **The inspector's thresholds and sentences are placeholder content.** The mechanism
   is settled — the diagnosis comes from whichever measurement is furthest out of range,
   normalised against its own band so units stay comparable, and reads as a plain
   statement of what limits the machine when everything is in range. The bands and the
   copy depend on the element roster (spec §11 q7). Only the residence-low sentence is
   the designed copy.
4. **One notice at a time**, per the handoff. A queue, a stack and a counter are all
   plausible and none is chosen (handoff open question 3).
5. **Zoom is fixed at 4×.** The tile grid is drawn at one zoom and its behaviour past
   tile legibility is undesigned (handoff open question 4), so no zoom control exists.
6. **The status bar shows the last known cursor position** when the pointer leaves the
   viewport, rather than blanking. Not specified either way.

## Note on build order

Spec §10 puts the deterministic sim core first and everything else after it. The HUD
was built first by request, then the core. Nothing in the client constrains the sim —
the HUD reaches the world only through `SimSurface` and a readout struct — and nothing
in the sim knows the client exists.

Milestones 1, 2a and 2b have met their gates. What remains of Milestone 2: eviction once
§2.4's open question is answered (2c), and the WebGL2 renderer that finally connects the
two halves (2d).

Since then the vertical slice shipped and kept going, out of milestone order: the refine
chain (burner, compactor, fuel), belts and filters with sprite animation, machines that
can be switched off and retuned, a conducting heat field with a heater and an overlay to
see it, and gas — steam, glass, and phase changes that run both ways. Nine rounds. What
that produced, and why it is not yet a game, is
[`docs/WHY-ITS-A-TOY.md`](docs/WHY-ITS-A-TOY.md).

### Carried forward

Settled provisionally, and still open:

- **Scan order across chunk boundaries** never became a problem, because iteration
  stayed a single global sweep through both 2a and 2b. Chunks decide what is *skipped*,
  never what order the rest runs in. Anything that later makes chunks iterate
  independently — threading, most obviously — brings the question back.
- **Liquids never settle**, which is what stops sleeping from paying off. See §3.5; the
  test `liquid_worlds_never_settle` pins the current behaviour, and flipping it to
  `assert_eq` is the check that a future liquid model actually terminates.
- **Nothing holds the world up.** On an infinite canvas, material with nothing beneath
  it falls forever and allocates chunks as it goes (§3.6).
- **Eviction is blocked on a design decision**, not an implementation one. It makes
  world state depend on camera history, which collides with replay verification — see
  §2.4 of the spec.
- **Liquids spread one cell per tick** and never rise. Enough to level out and to let
  powders sink through, and slower than a real flow model; revisit when flow rate is
  something the game cares about.
- **Heat exists, and nothing in the factory reads it.** The field conducts, phase
  changes fire off it in both directions, and the overlay (`H`) shows it. But no
  machine's rate or output conditions on temperature, and `Reaction` has no temperature
  band — so heat is currently scenery. This is the central finding in
  [`docs/WHY-ITS-A-TOY.md`](docs/WHY-ITS-A-TOY.md).
- **Gas has one element.** `steam` is the only one, and it condenses back to water. The
  buoyancy rules were written for it rather than guessed at in the abstract, which means
  they are tuned against a single case.
- **The flat world is kept deliberately.** It is the oracle the chunked world is
  measured against, and its hard edges are what make "nothing escaped" a meaningful
  claim — on an infinite canvas there is nowhere for escape to be observed. It should
  outlive its apparent redundancy.
