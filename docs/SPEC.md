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

**Currency is matter too.** Gold was the last number standing in for stuff, and it is now
an element like any other: a press turns product into nuggets, the
nuggets are cells that fall and stack and can be buried or spilled, and the player's
balance is what a machine is holding (§5.1). Nothing anywhere converts matter into a
figure — the pile *is* the figure.

**Machines may still consume, and that is allowed.** A press destroys what it presses:
seven grains of an eight-grain nugget leave the world, and so do the nuggets themselves
when they are spent. That is a physics rule operating on matter in place, which is what
the pillar leaves room for. What it rules out is *summarising* — no offline accrual, no
throughput number standing in for particles, no sink that pays for matter it never
physically received.

The line between the two is accounting, so it gets a test rather than a promise: every
cell that leaves the world is either eaten as valueless input, pressed into a nugget, or
spent, pinned by `crates/sim-core/tests/press.rs`.

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

### 2.3a Are constructs matter? **[OPEN]**

`wall` is currently an element, so a drawn wall is cells in the particle grid that happen
never to move. That is how falling-sand games usually do structure, and it is simple: one
grid, and particles interact with walls through the same density rules as everything
else.

But it sits oddly against the rest of this document. §2.3 says the player draws
*constructs* onto a tile grid, §7.2 saves those constructs at tile level, and §4.1
already puts belts on a separate layer from the particle grid. On that reading a wall is
structure, not matter, and belongs with the belts rather than with the sand.

The distinction is not academic. It decides whether a wall can be buried, melted, or
displaced; whether saving a factory means saving cells or tiles; and whether "draw" and
"place a machine" are the same action or two different ones.

Cheap to leave as it is for now, and worth deciding before saving is built.

**Finding, from making walls tile-aligned.** *Placement* is now settled: a stroke is a
line over tiles and each one it touches is filled completely, so a drawn wall is always a
whole number of 9x9 tiles and building is one grid for machines and walls alike. That is
half of what this section was asking about, and the cheap half.

*Representation* is still open. A wall remains an element — cells that never move — and
nothing stops a future rule from melting or displacing part of one, at which point a tile
is partially filled and the world is off-grid again by another route. So the question is
no longer "should the player be able to draw off-grid" (no, and they cannot) but "may the
simulation take a wall apart cell by cell". Decide that with reactions, not before.

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

**Spawners emit matter that moves.** A solid emitter would place one cell and stop,
which is drawing rather than emission — structure is drawn, not spawned. Refused in the
simulation and not only in the interface.

**Gold buys capacity, not placement.** Raising the cap costs gold; where a spawner sits
within that cap is a layout decision, and picking one up refunds its slot in full — no
cost, no cooldown, no penalty.

Charging for a misplacement would push players to hoard slots and stop experimenting,
which is backwards for a game whose entire loop is iterating on machine geometry. The cap
exists to limit how much input you have, not to punish you for putting it in the wrong
place. This also removes a soft-lock: without removal, spending every slot on one
material leaves a run unwinnable.

The same applies to every placed machine, not only spawners.

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

**As built:** a belt's own tile footprint is filled, at placement, with a real,
indestructible structural element — solid ground the existing gravity/density rules
already know how to hold things up on. The belt's own logic only ever touches the row
of cells directly *above* its footprint: each tick it shoves whatever is resting there
one cell toward its declared direction (`Entity.direction`, `1` or `-1`), then physics
settles it back onto the belt's now-solid surface, the same way it settles anything
onto a floor. No entity-awareness was added to the generic physics sweep. Horizontal
only for now — open question 1 (§11), how material moves upward, is unresolved and a
vertical or inclined belt would need to answer it.

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
  destroys them. Built as a real element (state solid, so it never moves regardless of
  what is denser) rather than a rule anything has to special-case.

**Filters are belts with a hole.** A filter is a conveyor that lets one selected element
fall through its underside and carries everything else on. That is the whole mechanic —
no sorting logic, no inventory, no routing graph — and it is what turns a mixed stream
into directed material: run the output of a washer along a filter set to gold and the
nuggets drop into the vault below while the rest carries on.

**As built:** a filter is not a second behaviour — it is a belt entity whose instance
names which element to let through, the same instance field an emitter uses to say
which element it emits. One "filter" tool is tuned per placement, the same way one
"emitter" tool is; there is no separate machine type per material.

Worth noting what this implies for §5.1: today a vault has to sit directly under the
press, because gravity is the only transport. Filters make a vault somewhere else
possible, which is when a factory stops being one vertical column.

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

**A nugget is a particle, and a balance is a place.** A press banks the value of what it
takes and, every `goldPer` points, the cell it is working on becomes gold instead of empty
space — several grains in, one nugget where the last one was, which is pillar 2 as an
object rather than a curve. Nuggets fall, stack, and are subject to everything else in
the world.

**Only what a vault is holding is money.** A vault is a *place the player made*: dig a
pit, wall it, and mark the inside out with the vault tool. It is any size — the footprint
is whatever was dragged, not a fixed machine size — and it does nothing each tick. What
makes it work is only that gold inside one counts.

Nothing checks the walls. A vault marked over open ground is a perfectly legal vault that
happens to leak, exactly as a press with an open bottom is a legal press that happens to
leak. Physics decides whether the gold stays, not a validity rule.

**Density sorts a mixed pile on its own.** `displaces()` — the rule that already lets
sand sink through water — now applies between any two powders, not just powder-through-
fluid: a denser powder sinks through a lighter one, no element named in the rule itself.
Gold at 19300 sinks below residue (2200) and below sand and wet sand (1600–1900), which
is what stops a silted vault from staying an undifferentiated pile — panning is just this
rule running over time. `crates/sim-core/tests/vault.rs` pins it directly.

**Pressing and storing are separate jobs, and gravity connects them.** A **press** turns
product into nuggets; it holds them only until they can fall somewhere, so a press on a
solid floor fills with its own output and reports itself blocked. The build that works is
a press over an open pit that has been declared a vault.

**A press takes only what it can press.** Everything else — walls, water, unwashed sand,
and the nuggets it has already made — falls straight through and is none of its business.
That is what stops a machine from competing with the reaction feeding it: an earlier
version ate whatever entered it, which meant a fast one consumed the sand and water
before they could meet and a slow one let the raw material pour past into the vault.
`rate` is now a real throughput limit rather than a race — product arriving faster than
the press can work carries on falling.

Spending reaches into vaults only, takes the exact price or nothing, and the pile visibly
shrinks. A press never takes currency back, or it would grind its own output into nothing.

**Open, and waiting on §4.3: vaults contaminate.** Everything the press does not take
falls into the vault below it and stays there, so a heavy feed silts the vault up with
product and water instead of gold. Nothing is lost — it is all still matter — but a
vault packed with wet sand has no room for nuggets. Filters are the intended answer.
A second candidate is worth recording: letting a **denser powder sink through a lighter
one**, which `displaces` currently allows only for liquids and gases. Gold at 19300
would settle out of a mixed pile the way panning works, and the vault would sort itself.
That is a real physics change that moves the golden hash and touches §3.5, so it wants
its own round.

### 5.2 Sinks **[PROPOSED]**

Gold buys **capability**; raw materials build **machines**. Keeping these separate stops
gold from feeling like an arbitrary second resource.

Gold sinks: spawners, spawner upgrades, teleporters and teleporter range, blueprint
slots, copy/paste unlocks, late-game buildings.

**Built so far: spawner capacity, and nothing else.** One sink is legible where three
would be noise, and it is the one the design already argues for — §3.4 makes spawner
count the only hard limit on production, so widening it is the largest thing gold can do.
The price doubles per slot.

**There is no ledger, only the world.** The balance is measured — nuggets held by
machines — the same way contact area is measured, and paying is a physical act that takes
them out. Nothing tallies a balance anywhere, so nothing can disagree about one.

This is a better position for §8.1 than the ledger was. The simulation is the authority on
whether a purchase happened: a client claiming to have bought something has to produce a
world in which the nuggets were there and are now gone. What stays progression state is
only what the purchase *bought* — the spawner cap — and that is a small, checkable
number rather than a running total the server would have to trust.

### 5.3 Byproduct progression **[DECIDED]**

Production chains emit byproducts that are useless at the tier where they first appear
and become required inputs at a later tier.

Worked example (illustrative, not final): sand + water → wet sand → processed into gold
+ a side product. The side product has no use initially; a later tier requires it.

The retroactive payoff scales with chain depth. A byproduct from step 3 re-entering at
step 7 forces the player to rebuild a factory they considered finished.

**The mechanism is built; the roster is not.** A press converts, it does not destroy:
every cell it takes either completes a nugget or becomes **residue**, a real element
(`data/elements.json`) rather than `EMPTY`. Nothing gives residue value yet — that is
exactly the "later tier" this section describes, and it is deliberately not decided here.
What changed is that the worked example above is no longer illustrative; it is what the
press does.

This turned out to make pressing fully particle-conserving: eight physical cells in,
eight out — one nugget and seven residue, every time. The only place matter still leaves
the world outright is spending (§5.1), which is the one case §1.1 already licenses.

**The first chain is built: residue → burn → burnt residue → compact → fuel.** One
generic mechanism serves both steps — `Behaviour::Refine`, which turns up to `rate`
cells of one declared input element into whatever that element's `refined_into` names,
no banking or threshold the way pressing has, because there is nothing to accumulate
toward. A burner (`input: residue`) and a compactor (`input: burntResidue`) are the same
code with different data rows, exactly as burn and compact are the same shape with
different verbs. Both new elements ship inert, same as residue did: nothing burns fuel
yet.

*Superseded entirely.* Both machines are gone, and so is the behaviour they ran on:
`Behaviour::Refine`, `Reach`, `reach` and `refinedInto` were all deleted with them.
Burning and compacting are physics now — see **Burning is not a machine** and
**Compacting is not a machine either** below. The paragraph above is kept because it is
the reasoning that produced the chain, and the chain survived; only its machinery did
not.

**A machine also declares where it reaches.** *(Deleted along with `Refine` — kept for
the reasoning.)* `Refine` gained a `reach` field rather
than a second behaviour, the same way a filter turned out to be a belt with one more
field: a machine that takes what falls into it and a piston that crushes what is heaped
under it do the identical thing to whatever they find and differ only in where they
look. `body` is the machine's own footprint — material that
has fallen in, with gravity as the conveyor. `below` is the tile directly beneath it.

The compactor is `below`, and two tiles tall: a gantry over a ram that comes down on
whatever is heaped underneath. That makes it the first machine you feed by piling
material *under* it rather than dropping material *into* it, which is a different thing
to build around — and the first real animation, since the stroke is what the reach
looks like. Sprite rules gained a `when.row` matcher so a machine taller than one tile
can have a head and a body instead of one tile stamped twice; the ram's frames drive
the stroke, and the gantry it hangs from deliberately holds still, or the whole machine
would read as sliding.

**What fuel is for: a generic heater, not a boiler.** A machine that burns fuel next to
water to make steam was the first idea and the wrong scope — `data/elements.json`
already carries `melting_point`, `boiling_point` and `thermal_conductivity` on every
element, parsed and stored since Milestone 1, read by nothing. Heat is meant to be a
real simulated quantity other things key off, not a side effect of one machine. That is
its own round: what carries heat, how conductivity moves it, what crossing a melting or
boiling point does. Fuel sits ready for it, the same way residue sat ready for this.

**Heat is built, and it is a quantity rather than a machine's side effect.** Every cell
carries a temperature in whole Kelvin (`crates/sim-core/src/heat.rs`), stored parallel
to the element grid in both `Grid` and `ChunkMap`, and swapped along with the matter
when a cell moves — a falling grain carries its own warmth. `heat::step` runs once a
tick after the movement sweep, deliberately *not* folded into it: that sweep's
`is_moved` bookkeeping exists to stop a particle being carried twice, which has nothing
to do with heat.

- **Heat only exists where matter does.** It does not conduct through empty space, and
  an empty cell never reads as anything but ambient. That sidesteps diffusing across an
  unbounded empty canvas (§2.1), and is the more honest reading of conductivity anyway:
  two things exchange heat by touching, not by sharing a void.
- **Conduction is pairwise and capped at the midpoint.** Each occupied cell trades with
  its right and lower neighbour — the same "each adjacent pair considered exactly once"
  pattern reactions use — moving the temperature difference scaled by the *lower* of the
  two conductivities, clamped to half the difference. Using the lower is what makes an
  insulator insulate: `beltStructure` is `0.0`, so nothing conducts into or out of a
  belt's body, a second independent guarantee of §4.3's "lava does not melt them". The
  half-difference cap is load-bearing: gold conducts at `3.17`, and without it a good
  conductor would swap the two temperatures outright and flip which side is hotter.
- **Melting and boiling are one generic mechanism.** `meltsInto`/`boilsInto` name what
  an element becomes on reaching its `melting_point`/`boiling_point`, resolved in the
  same name pass the other linked element fields already use. Boiling is checked first: anything
  hot enough to boil crossed its melting point on the way. Sand melts into
  **moltenSand**, a real liquid that flows and settles immediately. `boilsInto` ships
  real but unpopulated — steam needs gas-state physics, which is still a no-op.
- **The heater banks nothing.** It burns up to `rate` cells of fuel from its body and,
  if it burned any, holds its footprint at `heatOutput`. Cooldown is free and not
  special-cased: it simply stops forcing the temperature, and the same generic
  conduction pass carries the heat away.

**Burning is not a machine, and the kiln is what makes that buildable.** Heat shipped,
and then nothing in the factory read it: no machine's rate or output conditioned on
temperature, and `Reaction` had no temperature term at all. A full thermal field with
phase changes, and it was scenery. The fix is not another machine — it is deleting one.

- **Residue burns where it is hot enough.** `burnsInto` names the product and
  `ignitionPoint` the threshold, resolved in the same name pass the other six linked
  fields use. Unlike melting, it is one-way and *probabilistic*: crossing the threshold
  only makes a cell eligible, and `flammability` — a field parsed and stored since
  Milestone 1 and read by nothing until now — is the chance per tick that it converts.
  So **how long a cell spends hot is what sets the rate**, which makes residence time a
  consequence of the layout rather than a number on a machine. That is the second of
  §3.3's five yield factors to become real, after contact area.
- **Burning never falls through to melting.** Anything hot enough to melt passed its own
  ignition point on the way up, so a failed roll must not melt instead. Residue's
  ignition point (800K) sits well clear of water's 373K boiling point in one direction —
  so a burn run and a wash cannot share a space — and well under its own 1500K melting
  point in the other.
- **The kiln is `belt` with one field changed.** `chassis: kilnStructure` instead of
  `beltStructure`, and `convey`/`convey_one` are untouched. Cargo rides the row directly
  above a belt's body, so on a kiln it rests on a `2.0` conductor rather than a `0.0`
  one: a heater set against the run heats everything the run is carrying, and **belt
  length is dwell time** while **heaters are temperature**. Both are things the player
  draws rather than sets.
- **Nothing stops to be processed.** This is why a kiln rather than a hot basin. Heat
  needs contact and contact needs time, but a pile that has to sit somewhere hot is a
  jam waiting to happen; a conveyor gives dwell without ever holding material still.
  `beltStructure`'s `0.0` becomes a real choice rather than trivia — the plain belt is
  now the deliberately *insulated* one.
- **A heater must be fed continuously, not merely stocked.** `burn_fuel` empties its
  footprint from the bottom up, so the top row — the face that touches whatever sits on
  it — hollows out first. A heater left to burn down stops heating long before it stops
  having fuel. That is a logistics requirement rather than a bug, and it is what makes
  routing fuel *to* the heat part of the puzzle.

**Compacting is not a machine either, and pressure is the quantity it reads.** The open
question left by the burner — whether the compactor should follow it out — resolved yes,
and the mechanism it needed turned out to be the third of §3.3's five factors.

- **Load is the weight of the column standing on a cell.** The contiguous run of matter
  directly above it, summed by density (`compress.rs`). A gap resets it to nothing: a
  shelf with air under it presses on nothing, which is both the honest physical reading
  and what keeps the chunked and flat worlds agreeing, since above the topmost matter
  every column is empty however far its bounds reach.
- **Derived, never stored.** Unlike temperature, load is recomputed every tick from the
  arrangement of matter alone — a cell's temperature is history, its load is only where
  it is right now. So there is no third parallel grid, nothing to swap when matter
  moves, and nothing added to the save format. One top-down sweep carries a running
  total per column, so the whole field costs a single pass rather than a walk up the
  column at every cell.
- **`compactsInto` and `compactionLoad`, with `hardness` as the resistance.** The exact
  mirror of `burnsInto`/`ignitionPoint`/`flammability`, on the other quantity — and
  `hardness` is the third field parsed and stored since Milestone 1 and read by nothing
  until the round that needed it. The chance per tick scales with how far past the
  threshold the load is, so a deeper pile compacts *faster* rather than merely
  compacting at all: **depth is a dial with a range**, not a switch with two positions.
- **All integer, and never through `Fixed::from_int`.** A load runs to tens of thousands
  and Q16.16 tops out near 32767, so the ratio is taken first in i64 and only the ratio
  is ever a Fixed quantity. The accumulator is i64 because an unbounded canvas puts no
  ceiling on column height and `beltStructure`'s density is 999999.
- **Weight is weight, wherever it comes from.** A lid of structure piled on a shallow
  charge pushes it over the line, so "build a heavier roof" is a real alternative to
  "dig a deeper hole". Nothing special-cases where the load originates.

**The machine layer is now transport, storage and energy — never conversion.** What is
left is the emitter (input), the press (the one licensed sink), the vault (storage), the
belt, the filter and the kiln (transport), and the heater (energy). Every conversion in
the chain past the press reads a physical quantity instead: contact area for the wash,
temperature for the burn, load for the compaction. Three of §3.3's five factors are real,
and all three come out of what the player drew.

**[OPEN]** The press is still a black box, and so is the wash's fixed probability. The
press is arguably fine — it is a sink rather than a conversion, and §1.1 licenses it —
but the wash reaction having no temperature term is the next obvious gap.

**[OPEN]** The rest of the element roster and tech tree beyond this one chain.

**[OPEN]** Heat is not gated by chunk sleeping — `heat::step` walks full bounds every
tick. Folding it in means deciding whether "still conducting toward equilibrium" counts
as quiescent, which today it would not. No live regression, since the client never
enables sleeping, but a real gap once it does.

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

**Gap, found while building the press.** Nothing about the entity layer is in any
hash. `canonical_hash` covers cells, tick and seed; machines, their placement, and the
part-nugget each press has banked are all outside it. Two replays could therefore
differ in machine state and agree on every checkpoint, which is precisely the case this
section exists to catch.

Currency being matter shrinks this problem without closing it: the balance itself is
cells now, so it *is* hashed, and only the banked remainder and the machine layout are
not. The fix is to fold entity state into the checkpoint hash, and it is cheap — worth
doing before checkpoints are signed rather than after.

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
