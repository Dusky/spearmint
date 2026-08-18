# Vertical slice — scope

**Status:** built, unplayed. All six steps below are done; what is missing is the
evidence they were built to produce. Deliberately cuts across the milestone order in
[`SPEC.md`](SPEC.md) §10.

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
| 6 | **Collector + gold** | Closes the loop and gives routing a purpose. |

Steps 1–3 already produce a partial answer. If drawing into a live sim is unpleasant,
stop there.

**Built.** All six. The loop runs: draw a basin, spawn sand and water, watch them mix,
and a collector set into the floor takes the product out for gold that buys more spawner
capacity. Two things the build settled that the scope did not anticipate:

- **A machine is not matter, so nothing rests on one.** A collector is fed by what falls
  *into* it, and an open-bottomed one lets product fall straight through. Building it a
  pocket of wall is the player's job, and getting that wrong is visible.
- **Only product pays.** Value is a property of the element (`data/elements.json`), so a
  collector under an unmixed stream destroys raw sand for nothing. Routing has a cost
  for being wrong, which is what makes it a decision.

What remains is the part no amount of building substitutes for: sitting down with it and
finding out whether tuning a washer is satisfying or fiddly.

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
- **Is the washer buildable at all?** The design handoff shows one at 68% yield with a
  residence problem. Nobody has built one. If the physics cannot produce a satisfying
  washer, the flagship example is fiction and we need to know.
