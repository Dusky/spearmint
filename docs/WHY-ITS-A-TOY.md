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

*(As of §6 the burner is gone and the heater's output is consumed. The other two rows
still stand.)*

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

**8 machines** — `emitter`, `press`, `vault`, `compactor`, `belt`, `filter`, `heater`,
`kiln`. All 1×1 except the compactor, which is 1×2. The burner was one of these and was
deleted in §6; its id (4) is retired, not reused.

**1 reaction** — `sand + water → wetSand + wetSand`, probability `0.04` per adjacent
pair per tick. That is the entire reaction table. Burning is *not* in it: it is a
threshold on the element, not a pair of reactants.

**1 gold sink** — the spawner slot in the capability drawer (`spawnersMax → +1`).
`state.upgrades` is `[]` (`src/state/initialState.ts:151`), so the drawer renders one
purchasable row and nothing else.

**Phase changes** — `sand ⇄ moltenSand → glass`, `water ⇄ steam`. Both directions run.
They are the only rules in the game that read temperature.

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

**Two of §3.3's five factors are now real** — contact area and residence time — and both
come out of the layout rather than a machine's settings. The conflicting-band puzzle
§4 wanted came free: water boils at 373K and residue ignites at 800K, so a wash and a
burn run cannot share a space.

### What this does not settle

The compactor is still a black box, and so is the press. Whether the column is really
gone is a question for playing it, not for arguing about it — the same standard §1 held
the last nine rounds to. The specific thing to watch for: whether routing fuel *to* the
heat, and keeping a kiln run hot enough for long enough, is a decision or a chore.

One finding worth carrying in either case: **a heater must be fed continuously, not just
stocked.** `burn_fuel` empties its footprint bottom-up, so the face that touches what it
is heating hollows out first, and a heater left to burn down goes cold long before it
runs out of fuel. That is a logistics requirement, and it is most of what makes the
layout non-trivial.
