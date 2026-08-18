# Particle Factory — Design & Technical Spec

**Status:** revision 2, post-review. For handoff to a Claude Code session.

Every item is tagged:

- **[DECIDED]** — settled by the designer. Do not change without asking.
- **[PROPOSED]** — suggested but not ratified. Flag before building on it.
- **[OPEN]** — genuinely undecided. Ask before implementing.

---

## 1. Concept

An incremental factory game built on a falling-sand particle simulation, viewed
**side-on** (Terraria-style). The player hand-builds machines that exploit particle
physics — gravity, heat, flow, density, phase change, reactions — to refine raw matter
into products. Products sell for gold; gold buys capability, which enables deeper
production chains.

The reference points are Powder Toy for the physics and Factorio for the production loop,
but the design is deliberately *not* Factorio's: there is no menu of prefab machines. A
machine is something you drew.

### 1.1 Core pillars **[DECIDED]**

1. **Everything is hand-made.** The player builds machines out of drawn constructs.
   Progression does not consist of unlocking better prebuilt machines.
2. **Progression is compaction.** Rewards make your factory denser, more repeatable, and
   less tedious to extend — not automatically more powerful.
3. **Input is capped, output is skill.** The number of particle spawners is the hard
   limit. Getting more out of the same input is the entire game.
4. **Byproducts become ingredients.** Waste that is useless in one era is a required
   input in the next.
5. **Particles are never abstracted.** No system anywhere converts matter into a number.
   Particles are not created, destroyed, or summarised outside of the physics rules
   themselves. This constraint is load-bearing — see §2.4 and §6.

---

## 2. World model

### 2.1 Canvas **[DECIDED]**

The world is infinite in extent. Space is not a scarce resource.

### 2.2 Perspective **[DECIDED]**

Side-on, like Terraria. Gravity is a first-class mechanic and does vertical transport
downward for free. Belts move material horizontally.

**[OPEN]** How material moves *upward*. This is a real problem and a natural progression
axis — elevators, pistons, pressure, pumps, or falling-loop designs. Needs an answer
before belts are built.

### 2.3 Tile grid **[DECIDED]**

The player builds by drawing **constructs** onto a tile grid. One tile is approximately
**9×9 particle cells**. A conveyor segment occupies one tile.

Odd tile dimensions are intentional: every tile gets a true center cell, which matters
for rotation, symmetry, and teleporter endpoints.

**[OPEN]** Final tile size.

### 2.4 Chunking and sleeping **[DECIDED]**

Chunks are defined in **tiles, not pixels**, so tile/chunk alignment never drifts.
Suggested starting point: 16×16 tiles per chunk. Tune after profiling.

Chunks are an internal optimization and are **never surfaced to the player** — no grid
lines, no chunk counters, no purchasable chunk limits.

**Sleeping is freezing, not abstraction.** A sleeping chunk retains its exact particle
array in memory and simply stops ticking. Waking resumes precisely where it left off. No
particle is invented, destroyed, or converted to a rate. This costs RAM, not CPU.

Sleeping is gated on **distance from the viewport**, not merely on steady state. Anything
the player can see, or could see imminently, stays live. The player must never be able to
notice a sleeping chunk.

**Steady state is the condition that makes sleeping safe; the viewport gate is
belt-and-braces.** A chunk may sleep only when it is quiescent — nothing moved in it or
in any neighbour last tick — because ticking a settled chunk produces no change, so
skipping it produces no difference. The viewport gate sits on top and can only keep
*more* chunks awake, so it spends CPU and can never alter the world. Sleeping therefore
does **not** inherit eviction's replay problem below: it is unobservable, where eviction
is not.

**As of Milestone 2b this is implemented and proven, and delivers nothing.** Liquids
never come to rest (§3.5), so any chunk holding water stays dirty and keeps its
neighbours awake. Powder-only regions do sleep. The optimisation is gated on liquid
settling, not on more work here.

**[OPEN]** Eviction policy. Frozen chunks accumulate RAM without bound on an infinite
canvas. At some distance threshold a chunk must be discarded entirely and re-settle from
empty on return (consistent with §7.1). Decide the threshold and whether eviction is
distance-based, LRU, or memory-pressure-based.

**Eviction makes world state a function of camera history, and that collides with replay
verification (§8.3).** Sleeping does not — a sleeping chunk resumes exactly — but an
evicted one re-settles from empty, so two players with identical action logs diverge if
one wandered far enough to trigger eviction and the other did not. All three candidate
policies inherit this: distance-based and LRU both depend on where the player looked,
and memory-pressure-based makes the world depend on the player's hardware, which cannot
be replayed at all.

Whatever policy is chosen must therefore be a deterministic function of simulation state
and the logged inputs. That rules out memory pressure for anything the server verifies,
and leaves three coherent directions: log camera movement as a replay input, tie eviction
to something in-world rather than to the viewport, or scope verification to economy
aggregates rather than world hashes.

---

## 3. Simulation

### 3.1 Determinism — non-negotiable **[DECIDED]**

Same inputs must produce byte-identical output on every machine, every run.

- **Integer or fixed-point math only.** No floats anywhere in the sim.
- **Seeded PRNG owned by the sim.** No `Math.random`, no system entropy.
- **The PRNG is stateless and position-hashed**, not an advancing stream. Every draw is
  a pure function of `(seed, tick, x, y)`. A stream satisfies the bullet above and still
  breaks once chunks sleep (§2.4): a sleeping chunk consumes no random numbers, so every
  cell evaluated afterwards would draw a different value than it would have in a session
  where that chunk stayed awake, and the world would diverge based on where the player
  was looking. Hashing removes that, and any dependence on visit order or threading,
  by construction. *Settled in Milestone 1 — do not "optimise" it back into a stream.*
- **Fixed timestep.** Sim ticks are decoupled from render frames.
- **Deterministic iteration order.** No hash-map iteration, no unordered parallelism. If
  chunks update in parallel, update order and boundary resolution must be fixed and
  reproducible.

Determinism gates cross-device sync, replays, and all anti-cheat. Retrofitting it is
extremely expensive. It must be proven before anything is built on top of the sim.

### 3.2 Elements and reactions **[DECIDED]**

Element definitions and reaction rules live in **data files, not code**. Adding an
element means editing JSON, not writing a match arm.

Element schema fields: density, state (solid/powder/liquid/gas), melting and boiling
points, thermal conductivity, color and color variance, flammability, hardness.

Reaction schema: reactant set, product set, temperature range, probability per tick,
optional catalyst.

Two consequences of §3.1 that bite at the data layer rather than in the physics:

- **No JSON library may parse sim data.** Every one of them parses numbers into `f64`,
  which would launder element values through a float on the way into a sim that is not
  allowed to contain one. Decimal text is converted straight to fixed-point with integer
  arithmetic instead. The sim core therefore carries its own reader and has no
  dependencies.
- **Element ids are declared in the data file and are permanent.** They are baked into
  saves (§7.2) and world hashes (§8.3), so ids must never be derived from file order and
  a published id must never be reused for something else.

### 3.3 Reaction efficiency **[DECIDED]**

Reactions do **not** have fixed conversion ratios. Yield varies with layout —
temperature, mixing, residence time, contact area, pressure. Two players with identical
spawner counts get different throughput depending on how well they engineered.

This is what prevents the game from being solved. With capped input and fixed ratios,
there is one optimal build and the engineering ends.

### 3.4 Spawners **[DECIDED]**

Particle spawners are the sole hard constraint on production. The player has a limited
number; more are bought with gold. Spawner count, spawn rate, and output purity are all
plausible upgrade axes.

### 3.5 Liquid settling **[OPEN]**

Liquids as implemented never reach a fixed point, and this blocks chunk sleeping (§2.4)
from being worth anything.

A void trapped inside a body of water random-walks forever. Water never rises, so the
void cannot escape upward; a lateral move costs nothing, so water shuffles around it
indefinitely. Powders settle correctly — every move strictly descends, so they must
terminate — but any region holding water churns for as long as the world runs.

**Density: fixed.** Water used to saturate at about 76% in a stable lattice of holes,
which read as a rendering fault rather than as a fluid. The cause was that submerged
liquid kept flowing sideways, and each lateral move left a void that gravity refilled at
the same rate voids were created. Liquid now only flows sideways at the surface, which
is where spreading actually happens. A body of water packs solid, still finds its level,
and the washing reaction's yield more than doubled — dense water simply touches more
sand. Pinned by `crates/sim-core/tests/liquids.rs`.

**Termination: still open.** The surface continues to churn, so a world holding water
never reaches a fixed point and chunk sleeping (§2.4) still cannot engage for it.

**Dispersion: done.** Surface liquid now scans several cells sideways and takes the
furthest it can reach, stopping early where it could fall instead — the genre-standard
approach. Water levels essentially exactly across a tank rather than lagging behind the
pour, and it reads as a fluid.

What is left of this question is narrower than it looked. Density and levelling are
solved by following what the genre already does; only termination remains, and a
pressure field is the usual next step.

Still genuinely undecided: whether pressure becomes a real field, given §3.3 already
counts it among the things reaction yield depends on.

The naive fixes each break something:

- *Only move sideways toward somewhere it can fall* — a column of water on a flat floor
  then never spreads at all.
- *Only surface liquid flows* — a partly-filled level channel still oscillates.
- *Require a drop, and let only surface cells move* — water spreads to one cell deep and
  then stops advancing, so a container never fills evenly.

Getting both spreading and termination is the actual problem, and it is a design
decision rather than an implementation detail: §3.3 already counts pressure and
residence time among the things reaction yield depends on, so whatever model is chosen
has gameplay consequences. A pressure or flow-field model is the usual answer.

Until it is decided, sleeping engages only for powder and empty regions.

### 3.6 What holds the world up **[OPEN]**

The canvas is infinite (§2.1) and gravity is real (§2.2), so material with nothing
beneath it falls forever, allocating chunks as it goes and never settling. Nothing in
this document says what stops it.

The save format lists a world seed (§7.2), which implies generated terrain, and terrain
would answer this. It needs stating either way, because "everything not standing on
something falls out of the world indefinitely" interacts badly with both chunk eviction
(§2.4) and the requirement that machines be self-starting (§7.1).

---

## 4. Belts and transport

### 4.1 Belt layer **[DECIDED]**

Belts are a **separate entity layer** from the particle grid — they are not themselves
simulated matter. But they carry **real particles**, moved by their own physics.

### 4.2 Belt semantics **[DECIDED]**

A belt moves actual piles of particles along its surface. Nothing is converted to a
count, a quantity, or a material type. What sits on the belt is exactly what will come
off the other end.

**Clogging is emergent, not a rule.** When the downstream end can't accept material,
particles physically pile up on the belt and back up along it. No special-case code
required.

### 4.3 Belt interaction **[DECIDED]**

- **Loading:** you load a belt by placing particles on top of it. There are no inserters
  or loader entities.
- **Burial:** particles can bury a belt, blocking it.
- **Destruction:** belts are **indestructible**. Lava does not melt them; nothing
  destroys them.

### 4.4 Teleporters **[DECIDED]**

Short-range point-to-point transport that removes the belt run between two points.

- Initial range: **a few cells only**
- **Straight line only**
- Range is upgradeable, but progression must be **slow and expensive**

**[OPEN]** Exact starting range, cost curve, upgrade granularity, whether diagonal or
turned teleports ever unlock.

### 4.5 Blueprints and copy/paste **[DECIDED — mechanic; OPEN — fidelity]**

Both exist and are bought with gold. They are, with teleporters, the progression spine.

**[OPEN]** Paste fidelity. Hand-built particle machines are fragile — they depend on local
temperature, feed timing, and neighbor state. A pasted copy next to a hot neighbor may not
behave like the original. Two coherent answers:

- **Blueprints just work.** Pasted machines are normalized to behave identically. Simple
  and forgiving, but weakens the physics premise.
- **Physics actually matters.** Pasted machines are subject to local conditions, making
  isolation, insulation, and thermal management real engineering problems.

These cannot both be true. Pick one before implementing blueprints.

**[OPEN]** Blueprint slot limits; whether blueprints are shareable between players.

---

## 5. Economy

### 5.1 Currency **[DECIDED]**

**Gold** is the spendable currency, produced by processing refined products.

### 5.2 Sinks **[PROPOSED]**

Gold buys **capability**; raw materials build **machines**. Keeping these separate stops
gold from feeling like an arbitrary second resource.

Gold sinks: spawners, spawner upgrades, teleporters and teleporter range, blueprint
slots, copy/paste unlocks, late-game buildings.

### 5.3 Byproduct progression **[DECIDED]**

Production chains emit byproducts that are useless at the tier where they first appear
and become required inputs at a later tier.

Worked example (illustrative, not final): sand + water → wet sand → processed into gold
+ a side product. The side product has no use initially; a later tier requires it.

The retroactive payoff scales with chain depth. A byproduct from step 3 re-entering at
step 7 forces the player to rebuild a factory they considered finished.

**[OPEN]** The actual element roster and tech tree. Needed before content work, not
before framework work.

### 5.4 Byproduct accumulation **[OPEN]**

With an infinite canvas, dumping waste is free — the player can build a slag desert
offscreen and forget it. That is acceptable, but it means byproduct storage is not a
design constraint, and the later "you already have a mountain of it" payoff has to happen
naturally rather than being engineered.

Since particles are never abstracted (§1.1) and belts clog physically (§4.2), unwanted
byproducts *will* back up into machines and halt production unless the player routes them
somewhere. Confirm this is intended, and make sure the first occurrence is legible and
survivable.

### 5.5 Late-game buildings **[OPEN]**

Established: they exist, they are very late, they are very expensive, and they are
**infinitely upgradeable**.

What they actually *are* is undefined. A previous proposal (paying to freeze a machine
into a fixed-rate object) was rejected — machines stay live. Do not reintroduce it.

---

## 6. Offline progress **[DECIDED — deferred]**

**Not in v1.** Deferred, not cancelled.

Rationale: offline accrual requires converting production into a stored number, which
directly violates §1.1. Resolving that needs a deliberate decision (storage containers
holding counts, or gold-only payouts), and it is not worth blocking the framework on.

When it is revisited, the intended cap is **24 hours**, itself upgradeable.

**Consequence for implementation:** because there is no offline accrual, sleeping chunks
need no rate measurement at all. Sleeping is purely "stop ticking." Do not build
throughput measurement infrastructure — it is currently unused.

---

## 7. Persistence

### 7.1 Re-settling **[DECIDED]**

Live, mid-flight particles are **not** persisted to disk. On load, machines re-settle
from empty.

Note the distinction from §2.4: *sleeping* is in-memory and preserves particle state
exactly; *saving* does not. A chunk that is evicted or a game that is reloaded starts
that region from empty.

**Consequence, and it is a real one:** every machine must be **self-starting**. Any design
requiring a manual priming step is unbuildable. This is a legitimate constraint but it
must be communicated to players, not discovered.

### 7.2 Save contents **[PROPOSED]**

A save consists of:

- World seed
- Player-placed constructs, belts, teleporters, spawners (tile-level)
- Economy state (gold, spawners owned, upgrades purchased, blueprints)

Not saved: live particle state, belt contents in flight.

---

## 8. Client / server split

### 8.1 Authority **[PROPOSED]**

Full server-side simulation is not affordable — a live particle sim per concurrent user
does not scale. Split it:

- **Server owns the economy.** Gold, spawner count, upgrades, blueprints, progression
  state. Low-frequency, cheap to validate, and the only state that matters for progress.
- **Client owns the physics.** It reports production; the server decides whether to
  believe it.

### 8.2 Cheap anti-cheat **[PROPOSED]**

Because spawners cap input, there is a **provable maximum output**. The server knows the
player's spawner count and the theoretical conversion ceiling. Anything above
`spawners × max_rate × 100% efficiency` is impossible and is caught with arithmetic — no
simulation, no replay. This catches the large majority of cheating for free.

### 8.3 Replay verification **[PROPOSED]**

For the remainder: the client logs its actions with tick numbers. The server re-runs the
sim only when needed — on a leaderboard submission, on a random spot-check of a small
percentage of players, or when reported numbers look anomalous.

The **same Rust sim core** compiles to native for the server and WASM for the client, so
verification uses genuinely identical physics. Periodic signed checkpoints (world state
hash + resource totals) mean the server never replays from tick zero.

This makes the sim core's isolation a hard architectural requirement: no rendering, no
platform APIs, no I/O inside it.

### 8.4 Leaderboards **[OPEN]**

A raw "highest gold" board measures hours played, not skill, in a game with infinite
upgrades.

Suggested alternative: **seeded challenge boards** — same world seed, same spawner
budget, most output in N ticks. A pure engineering contest, and exactly what the replay
system verifies most naturally.

### 8.5 Accounts **[DECIDED]**

User accounts are planned, not required for v1. The architecture must not preclude them:
keep economy state serializable and server-shaped from the start, even if v1 stores it
locally.

---

## 9. Tech stack **[PROPOSED]**

| Layer | Choice | Rationale |
|---|---|---|
| Sim core | **Rust → WASM** | Cellular automata are branch-heavy; Rust is fast, and the same crate compiles native for server-side verification |
| Rendering | **WebGL2**, texture upload from a shared typed array | Well-supported and sufficient. WebGPU is faster but falling-sand is order-dependent and awkward to parallelize — not worth the complexity for v1 |
| Game logic / UI / economy | **TypeScript** | Iteration speed where determinism isn't required |
| Sim ↔ JS boundary | Shared typed-array view | Avoids per-frame copying |
| Element & reaction data | **JSON** | Content pipeline stays cheap |
| Backend | TBD | Only needs account storage, save sync, and occasional verification |

Target: browser-playable.

---

## 10. Build order

The first session should produce **the sim core alone**. Headless. No rendering, no
economy, no UI, no belts.

**Milestone 1 — deterministic sim core**

1. Fixed-point cell grid, typed-array backed
2. Seeded PRNG
3. Fixed timestep tick loop
4. Three elements only: sand (powder), water (liquid), wall (static)
5. Data-driven element loading from JSON
6. **Determinism test:** run 10,000 ticks twice from the same seed and assert identical
   output hashes. Then run it in a separate thread or process and assert the same hash
   again.

Do not proceed until that test passes reliably. If determinism is not locked down and
proven before other systems land on top of it, the server-authority plan quietly dies and
the cost of recovering it grows every week.

**Milestone 2** — chunking, dirty-rect updates, viewport-gated sleeping, eviction, rendering

**[PROPOSED]** Split Milestone 2, so chunking gets its own gate before anything is built
on it — the same shape as Milestone 1, and for the same reason.

- **2a — chunked storage.** Sparse chunks, deterministic ordering, boundary resolution.
  Gate: a chunked world and an unchunked one must produce identical hashes tick for
  tick. Keeping the flat grid as a test oracle is what makes that gate possible.
- **2b — dirty rects and viewport-gated sleeping.** This is where determinism is
  genuinely at risk, because what gets visited stops being a function of the world
  alone. Gate: a world where regions sleep must match one where nothing sleeps.
- **2c — eviction.** Blocked on the open question in §2.4, which is a design decision
  and not an implementation detail.
- **2d — the WebGL2 renderer**, which connects the sim to the client for the first time.

Rationale: 2a cannot break determinism if iteration stays a global sweep and chunking is
only storage, whereas 2b can. Landing them together makes a divergence expensive to
bisect.
**Milestone 3** — tile grid, constructs, drawing tools
**Milestone 4** — belts (real particle transport), burial, loading by placement
**Milestone 5** — spawners, gold, teleporters, first production chain

---

## 11. Open questions

1. How does material move **upward**? (§2.2) — blocks belt design
2. Final tile size (§2.3)
3. Chunk eviction policy and threshold (§2.4) — constrained: it must be deterministic
   from simulation state and logged inputs, or replay verification breaks
3a. How liquids find their level and come to rest (§3.5) — blocks chunk sleeping from
   delivering anything
3b. What stops material falling forever on an infinite canvas (§3.6)
4. Teleporter starting range, cost curve, upgrade granularity (§4.4)
5. Paste fidelity: do blueprints normalize, or does local physics apply? (§4.5) — blocks blueprints
6. Blueprint slot limits; sharing between players (§4.5)
7. Element roster and tech tree (§5.3)
8. Confirm byproducts backing up into machines is intended (§5.4)
9. What late-game buildings actually are (§5.5)
10. Leaderboard format (§8.4)
11. Backend platform and hosting

---

## 12. Explicitly rejected

Do not reintroduce these:

- **Simulation-budget / "awake cell" limits** as a player-facing constraint. Rejected
  because it makes the optimal move "build less," which is the wrong instinct for a
  factory game. Spawner count is the limit.
- **Crystallization** — paying gold to freeze a machine into a fixed-rate, non-simulated
  object. Rejected. Machines stay live.
- **Prefab buildings as the core loop.** Buildings are a very-late-game, very expensive
  exception, not the progression spine.
- **Abstracting particles into quantities** anywhere — on belts, in machines, or in
  storage. Belts carry real particles. See §1.1.
- **Inserters / loader entities.** Belts are loaded by placing particles on them.
