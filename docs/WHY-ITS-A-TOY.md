# Why it's a toy

**Status:** diagnosis, recorded after nine rounds of feature work and the first sessions
of actually playing the thing. **The finding in §1 has since been acted on** — see §6.
The diagnosis is kept as written rather than edited into agreement with what came after,
because it is the reason the work happened and it should stay falsifiable.

Original status line: diagnosis, recorded after nine rounds of feature work and the
first sessions of actually playing the thing. The verdict from playing it is *"it's a toy, not a
system"* — the pieces work individually but do not compose into decisions. This document
is the mechanical reason for that, and one proposal for fixing it.

This is a finding, not a plan. The proposal in §4 is marked **[PROPOSED]** and should be
weighed fresh, not inherited.

## 1. The finding

**We built a full thermal field, made it visible, and gave it phase changes — and
nothing in the factory cares what temperature it is at.** Heat is scenery.

Two greps settle it.

### 1.1 Not one machine reads temperature

`crates/sim-core/src/entities.rs` mentions temperature exactly twice:

```
402:    /// The whole-Kelvin temperature a `Heater` holds its own footprint at while it has
896:            field.set_temperature(x, y, definition.heat_output);
```

Both are the heater *writing* `heat_output` to its own footprint. No machine's rate,
output, throughput or behaviour conditions on how hot it is. The read side is empty.

### 1.2 Reactions have no conditions at all

`crates/sim-core/src/reactions.rs`:

```rust
pub struct Reaction {
    /// The two elements that must be adjacent.
    pub reactants: [ElementId; 2],
    /// What each becomes. [...]
    pub products: [ElementId; 2],
    /// Chance per adjacent pair per tick, 0..1.
    pub probability: Fixed,
}
```

A flat probability per adjacent pair per tick. Nothing else. `step.rs::react` says so
itself, in a comment written before heat existed:

> Note what is *not* here: no temperature gate, no catalyst, no rate table. Yield is
> whatever the geometry produces, because a reaction can only happen where reactants
> actually touch (spec 3.3). Contact area is not a parameter — it is the mechanism.

That comment was a correct statement of a deliberate design position when it was
written. It is now a description of the problem.

## 2. Why that makes it a toy

Every machine is a one-tile black box with one input and one fixed output:

| Machine | In | Out |
|---|---|---|
| press | wetSand | gold + residue |
| burner | residue | burntResidue |
| compactor | burntResidue | fuel |
| heater | fuel | heat — which nothing consumes |

*(As of §6 the burner and the compactor are both gone, and the heater's output is
consumed. Only the press row still stands.)*

A chain of fixed single-input transforms, with gravity as free lossless transport, has
exactly **one** correct layout: a vertical column. Stack them in dependency order and
let gravity do the routing. That column was built twice in a browser during the play
session and worked identically both times. There was no decision to make.

This is the thing spec §3.3 exists to prevent, arriving through a side door:

> Reactions do **not** have fixed conversion ratios. Yield varies with layout —
> temperature, mixing, residence time, contact area, pressure. [...] This is what
> prevents the game from being solved. With capped input and fixed ratios, there is one
> optimal build and the engineering ends.

Of the five factors §3.3 names, **one** is implemented — contact area — for **one**
reaction. Everything downstream of `wetSand` is fixed-ratio, which is precisely the
"one optimal build" case the spec rules out.

The wash reaction is the good part, and it is walled off. Contact area genuinely makes
the sand/water basin a layout problem, and it affects the *first* step only. Past that,
the pipeline has no choices in it.

## 3. What this is not

It is not a shortage of content. There are twelve elements, eight machines, phase
changes in both directions, belts, filters, sprite animation and a heat overlay. Nine
rounds of adding things produced a toy with twelve elements in it. A tenth round of
adding things produces a toy with fifteen.

It is not an architecture problem either. The sim is deterministic, chunked, sleeping,
proven equivalent to a flat oracle across processes, and the whole element and reaction
layer is data-driven. The machinery for the fix is already built.

The gap is that the physics and the economy do not touch. The sim knows temperature;
the recipes do not ask.

## 4. Proposal: condition recipes on the physics that already exists **[PROPOSED]**

Spec §3.3 already names what yield is supposed to depend on. Implement the second
factor.

**The cheap version.** Give `Reaction` a temperature band — `minTemperature` /
`maxTemperature`, both optional, defaulting to unbounded so every existing reaction
keeps its current behaviour. `react` gates on the cell's temperature before rolling
probability.

That single change makes *where the heater goes* a puzzle. It makes insulation matter.
It turns `beltStructure`'s `thermal_conductivity: 0.0` from trivia into a tool — the one
material that will not carry heat becomes how you wall a hot zone off from a cold one.
And it turns the heat overlay from a light show into an instrument, because it is
suddenly showing you the thing that determines your yield.

**The version that creates a real layout puzzle.** Two recipes in one chain with
*conflicting* bands. One step wants hot, the next wants cold. Now you must physically
separate them, and manage the gradient between them, in a world where heat conducts
through whatever you build out of. That is the Factorio-shaped decision the design is
missing, and it falls out of physics that is already simulated rather than needing a new
system.

**Why it is cheap.** Reactions are data (`data/elements.json`). The thermal field exists
and is deterministic. Phase changes already read temperature, so the pattern for
"a rule that conditions on heat" is written. The means to *see* the field exists (`h`).
The work is a struct field, a parse, a gate, and a recipe balance pass.

**Why it is left as a proposal.** It is a design decision with real consequences —
it makes every existing reaction's behaviour contingent on a number the player has to
manage, and a badly chosen band turns a working basin into a mystery. A session should
pick this up cold and be free to disagree with it. Alternatives worth weighing against
it: residence time (also named in §3.3, needs a machine boundary), pressure (blocked on
the §3.5 liquid model), or multi-input recipes (no physics dependency at all, and
therefore a weaker answer to the same problem).

## 5. State of the build

Derived from `data/`, so nobody has to derive it again.

**13 elements** — `structure`, `sand`, `water`, `wetSand`, `gold`, `residue`,
`burntResidue`, `fuel`, `beltStructure` (internal), `moltenSand`, `steam`, `glass`,
`kilnStructure` (internal).

**8 machines** — `emitter`, `press`, `vault`, `belt`, `filter`, `heater`, `kiln`,
`lift`. All one tile wide; the lift's height is the player's. The burner and the compactor were both deleted in §6; their ids (4 and 5) are
retired, not reused, and nothing shipped is taller than one tile any more.

**1 reaction** — `sand + water → wetSand + wetSand`, probability `0.04` per adjacent
pair per tick. That is the entire reaction table. Burning is *not* in it: it is a
threshold on the element, not a pair of reactants.

**1 gold sink** — the spawner slot in the capability drawer (`spawnersMax → +1`).
`state.upgrades` is `[]` (`src/state/initialState.ts:151`), so the drawer renders one
purchasable row and nothing else.

**Phase changes** — `sand ⇄ moltenSand → glass`, `water ⇄ steam`. Both directions run.
They were the only rules in the game that read temperature; burning (§6) is the second,
and the first one the factory depends on.

**2 physical conversions** — residue burns above `ignitionPoint`, burnt residue compacts
above `compactionLoad`. Neither is a machine. Both were, until §6.

## 6. What was done about it

The proposal in §4 was taken, and then pushed one step further than it was written.

**The cheap version was not enough on its own.** A temperature band on `Reaction` would
have left the burner standing — and a burner strictly dominates a heat-driven burn: it
is compact, deterministic, needs no fuel and no space. Nobody would ever have built the
alternative. So the burner was deleted, and burning became a property of the material:
`residue` declares `burnsInto` and an `ignitionPoint`, and `flammability` — dead data
since Milestone 1 — became the chance per tick that an eligible cell converts. Dwell
time is therefore the throughput dial.

**The conductor had to be the conveyor.** Heat needs contact and contact needs time, but
material that stops moving to be heated is a jam. The answer is the **kiln**: `belt` with
`chassis: kilnStructure` (2.0) instead of `beltStructure` (0.0), and `convey` untouched.
Cargo rides the row directly above a belt's body, so a kiln heats what it carries while
carrying it. Belt length is dwell; heaters against the run are temperature; the plain
belt becomes the deliberately insulated choice.

**Then the compactor went the same way, on pressure.** Load is the weight of the
contiguous column standing on a cell, derived every tick rather than stored — a cell's
temperature is history, its load is only where it is right now. `compactsInto` and
`compactionLoad` mirror the burn fields exactly, and `hardness` — the *third* dead field
from Milestone 1 — is the resistance. The rate climbs with how far past the threshold
the load is, so **depth is a dial with a range**, and a heavy lid works as well as a
deep hole because nothing cares where the weight came from.

**What that leaves is an architecture, not just two fixes.** The machine layer is now
transport, storage and energy — the emitter, press, vault, belt, filter, kiln and
heater. Not one of them converts anything except the press, which is a sink rather than
a conversion. Every step in the chain past it reads a physical quantity: contact area
for the wash, temperature for the burn, load for the compaction. Three of §3.3's five
factors are real, and all three come out of what the player drew.

The conflicting-band puzzle §4 wanted came free: water boils at 373K and residue ignites
at 800K, so a wash and a burn run cannot share a space.

### What this does not settle

**Whether the column is really gone is a question for playing it**, not for arguing
about it — the same standard §1 held the last nine rounds to. The vertical layout is no
longer free, because the two stages want incompatible shapes: a burn wants a long
horizontal hot run, a compaction wants a tall vertical pile, and fuel has to get from
the bottom of the chain back up to the heaters feeding the top of it.

**That last part was not possible when this was written.** Running the whole factory —
rather than each stage on its own rig — showed it compacting 110 cells of fuel and
starving its heaters to zero, because gravity goes down, belts go sideways, and nothing
moved a powder up. §7 is the fix, and the fact that three rounds of design missed it
until the thing was actually run is the most useful result in this document.

The press is still a black box, and the wash reaction still has a flat probability with
no temperature term — the most obvious remaining gap.

One finding worth carrying in either case: **a heater must be fed continuously, not just
stocked.** `burn_fuel` empties its footprint bottom-up, so the face that touches what it
is heating hollows out first, and a heater left to burn down goes cold long before it
runs out of fuel. That is a logistics requirement, and it is most of what makes the
layout non-trivial.

## 7. The loop did not close

Written after building the chain end to end instead of stage by stage — which should
have happened one round earlier, and is the whole method this document exists to argue
for.

**The failure.** An emitter dropping residue onto a heated kiln, its output falling into
a silo deep enough to compact: it worked. Residue burned, burnt residue compacted, 110
cells of fuel accumulated. And the heaters that make the burn possible ran to zero, with
110 cells of their fuel sitting at the bottom of a shaft they could not reach. Gravity
moves matter down, belts move it sideways, emitters only introduce it — **not one of
them decreases a cell's y.** The chain produced its own power source and could not use
it.

That is spec open question 1, *"How does material move upward?"*, which had sat as the
first item on the list since the beginning and had never blocked anything. Making the
conversions physical is what made it load-bearing.

**The lift.** A shaft whose column shifts up one cell per beat, stepping the top cell out
of the mouth — real particles, in order, as §4.2 requires of a belt. Two passes: one
every tick that cancels the cell gravity pulls down, and one on the beat that is the
actual climb. Its outermost columns are its walls, or the powder slumps out. A belt
underneath feeds it with no special case, because a belt's carry row *is* the bottom row
of a lift standing on it.

**What it cost to find.** Nothing, in code — the lift is a small machine. What it cost
was three rounds of confidently describing a loop that could not run. Each stage was
tested on its own rig and each stage worked; the composition was never tested, so the
one thing that was broken was the only thing nobody looked at.
