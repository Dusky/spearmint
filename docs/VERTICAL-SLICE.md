# Vertical slice — scope

**Status:** built, played, and it returned the falsifying result. All six steps below
are done, and several milestones' worth of work landed on top of them. Deliberately cuts
across the milestone order in [`SPEC.md`](SPEC.md) §10.

**The result is recorded in [`WHY-ITS-A-TOY.md`](WHY-ITS-A-TOY.md).** In short: the
pieces work individually but do not compose into decisions. Every machine past the wash
is a fixed single-input transform, and with gravity as free transport that chain has
exactly one correct layout — a vertical column. This slice was designed to surface
exactly this kind of answer, which is why it is worth keeping as a result rather than
losing.

## Why

Three milestones of foundation are done and proven. What none of them touched is whether
the game works. The architecture is sound; there is no evidence about the concept.

This slice exists to test one hypothesis:

> **Hand-drawing a machine and tuning it by watching physics is satisfying rather than
> fiddly.**

Everything else in the design is downstream of that being true. It can be tested without
chunk eviction, without a server, without belts, and without a WebGL renderer — so none
of those are in here.

The point is evidence, not progress. A result of "this is fiddly" is a **success** for
this slice, and is worth far more now than after another two milestones of
infrastructure.

## What the player can do

Draw a machine that washes sand, and watch a number respond.

1. **See** the world — the real Rust sim, running in the browser
2. **Draw and erase** walls at the tile snap, with `shift` for straight lines
3. **Place a spawner** that feeds sand and water continuously
4. **Watch** the two interact under gravity — no belts; gravity does vertical transport
   for free (§2.2)
5. **Read a real yield number**, derived from the sim, that moves when the drawing
   changes
6. **Collect** the product for gold

That is a closed loop: draw → run → measure → adjust → measure again. The loop is the
thing being tested, so nothing that isn't part of it is in scope.

## Build order

Sequenced so the riskiest and most informative parts come first.

| # | Piece | Why here |
|---|---|---|
| 1 | **wasm bridge** — `sim-wasm` crate, `sim-core` compiled to `wasm32` | The one genuinely unknown integration. If it is painful, better to know on day one. |
| 2 | **Renderer** behind the existing `SimSurface` seam | Gets the real sim on screen. Together with (1) this is the first time the two halves meet. |
| 3 | **Drawing** — wire `paint()` and `pickMaterialAt()` | Answers half the hypothesis on its own: does drawing into live particles feel good? |
| 4 | **Spawners** | Turns a static scene into a running process. |
| 5 | **Reaction + yield** — sand + water → wet sand | The actual novel claim: yield varying with layout, not fixed ratios (§3.3). |
| 6 | **Press, vault + gold** | Closes the loop and gives routing a purpose. |

Steps 1–3 already produce a partial answer. If drawing into a live sim is unpleasant,
stop there.

**Built.** All six. The loop runs: draw a basin, spawn sand and water, watch them mix,
and a press set into the floor turns the product into gold that buys more spawner
capacity. Two things the build settled that the scope did not anticipate:

- **A machine is not matter, so nothing rests on one.** A press works on what falls
  *into* it, and an open-bottomed one lets material fall straight through. Where it goes
  next is the player's problem, which is what makes the pit under the press worth
  building.
- **Only product pays.** Value is a property of the element (`data/elements.json`), and
  it doubles as what a press will touch at all — so an unmixed stream simply flows past
  one. Routing has a cost for being wrong, which is what makes it a decision.
- **Gold is a particle, and a balance is a place.** Currency started as a counter and
  became matter: a **press** turns product into nuggets and only gold inside a **vault**
  counts. A vault is not a machine — the player digs a pit, walls it, and marks the
  interior out at whatever size they like. Spending drains it. That closes the last gap
  between §1.1 and the economy, and it makes storage something you build.
- **A machine should not compete with the physics it depends on.** The press takes only
  what it can press. The version that ate anything in reach had two failure modes and no
  good setting between them: fast enough to keep up meant eating the sand and water
  before they could react, slow enough to leave them alone meant raw material pouring
  past into the vault.
- **A press converts, it does not destroy — and a mixed pile sorts itself.** Confirming
  Sandustry as the design reference (not just a comparable) paid off twice: what a press
  does not turn into a nugget becomes a real byproduct instead of vanishing, and a denser
  powder now sinks through a lighter one, the same physics that already lets sand sink
  through water. Between them a vault stops silting into an undifferentiated pile — gold
  settles to the bottom under whatever the press left behind.

**Played.** The part no amount of building substitutes for has now happened, and the
answer splits. Tuning the *washer* is satisfying: contact area is real, the yield number
responds to intent, and watching the particles explains why it is low. Everything
downstream of it is not a tuning problem at all, because there is nothing to tune — the
refine chain has no parameters the layout can move. See
[`WHY-ITS-A-TOY.md`](WHY-ITS-A-TOY.md).

## Decisions

**Render with a 2D canvas, not WebGL2.** §9 specifies WebGL2 and it is the right
long-term answer, but the hypothesis is not about rendering technology, and the
placeholder already proves the `ImageData` path works at this scale. `SimSurface` makes
the swap free later. Revisit if framerate becomes the thing under test.

**Raw wasm, no `wasm-bindgen`.** The interface is a handful of integer calls plus a
pointer into linear memory — `extern "C"` exports and a hand-written JS binding cover it
in roughly 150 lines, with no toolchain beyond `cargo build --target
wasm32-unknown-unknown`. Keeps `sim-core`'s dependency count at zero, which §8.3 asks
for anyway. Fall back to `wasm-bindgen` if the glue starts fighting.

**`sim-wasm` is a separate crate.** `sim-core` stays free of platform APIs, exactly as
§8.3 requires, so the same crate still compiles native for server-side verification.

**No TypeScript reimplementation of the physics, ever.** It would duplicate the rules,
discard the determinism work, and test physics the real game does not have.

**Do not fix liquid settling (§3.5) first.** Water that never comes to rest is a
performance problem, not obviously a visual one. Watching whether it reads as "alive" or
as "broken" is itself evidence, and turns an open question into an experiment. Fix it if
the slice says it looks wrong.

**Show only measurements that are real.** The inspector currently displays four
fabricated numbers. Yield and contact area can be computed honestly from the sim.
Residence time is derivable from Little's law — inventory over flow — within the
selection. Temperature has no heat system behind it and should be **removed from the
panel**, not faked. A smaller honest inspector beats the designed one filled with
fiction.

*Since superseded on one point:* the heat system was built, so temperature is back in
the panel and is measured. Residence and mixing are still absent, on the same grounds.

## Explicitly out

Belts, teleporters, blueprints, copy/paste, chunk eviction, the economy server, accounts,
leaderboards, replay verification, offline progress, save/load, onboarding, WebGL2, and
sound. Sleeping stays as built — it costs nothing to leave on.

Gravity-fed only. No horizontal transport, which is what lets belts stay out.

## What has to be true to call it a success

- Someone unfamiliar with it draws something that raises yield above the baseline,
  inside about fifteen minutes, without being told how
- The yield number responds **legibly** to a change — intent in, visible effect out,
  within seconds
- Watching the particles explains *why* yield is low, without a manual
- It holds 60fps at a world size big enough to build something interesting in

And the falsifying result, which is the one worth designing for: if tuning feels random,
if the number does not respond to intent, or if a player cannot tell why it is low, then
the core loop has a problem — and that is the most valuable thing this slice could tell
us.

## Salvage value

Most of this is real, not scaffolding. The wasm bridge, drawing, spawners, the reaction
system and the measurements are all things the finished game needs. Only the 2D renderer
is provisional, and it sits behind a seam built for replacing it.

## Questions this will force

- ~~**Does selling destroy matter?**~~ **Answered: yes, and it is accounted for.** See
  §1.1 — the pillar rules out *summarising*, not a machine consuming what falls into it.
  A test asserts every cell that leaves the world was collected.
- **What does a spawner look like to the player?** §3.4 makes spawners the entire input
  cap but says nothing about their form, footprint, or how output purity reads.
- **What holds the world up?** (§3.6) The slice needs a floor. A scene-level container
  works for now, but the real answer is terrain.
- ~~**Is the washer buildable at all?**~~ **Answered: yes.** It is the one part of the
  build that behaves the way §3.3 promises, because contact area is the one yield factor
  that got implemented. The problem turned out to be everything *downstream* of it —
  see [`WHY-ITS-A-TOY.md`](WHY-ITS-A-TOY.md).
