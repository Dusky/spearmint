# Particle Factory

An incremental factory game built on a side-on falling-sand particle simulation. See
[`docs/SPEC.md`](docs/SPEC.md) for the design and technical spec, and
[`design/particle-factory-ui/`](design/particle-factory-ui/) for the UI handoff this
client was built from.

**What exists today: the in-game HUD shell.** There is no simulation, no economy
server, and no belts. The chrome is real, typed, and pixel-faithful to the handoff;
the world behind it is a placeholder.

## Running it

```sh
npm install
npm run dev        # http://localhost:5173
npm run build      # typecheck + production build
npm run typecheck
```

Fonts are self-hosted, as the handoff requires. `node scripts/fetch-fonts.mjs`
regenerates `public/fonts/` and `src/styles/fonts.css` from Google Fonts.

## Controls

| Key | Action |
|---|---|
| `1`–`6` | Select tool |
| `shift` | Constrain a stroke to a straight line |
| `space` + drag | Pan |
| `alt` + click | Pick the material under the cursor |
| `F` | Jump to the current problem notice |
| `G` | Toggle the tile grid |
| `C` / `B` | Capability / Blueprints drawer |
| `Escape` | Close the drawer, then clear the selection |

`1`–`6`, `shift`, `space`, `alt` and `F` are from the handoff. `G`, `C`, `B` and
`Escape` are not — see "Flagged back to design" below.

## Layout

```
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

Spec §10 says the first session should produce the deterministic sim core alone,
headless, and that nothing should land on top of it until the determinism test passes.
This HUD was built first by request. Nothing here constrains the sim — the HUD reaches
the world only through `SimSurface` and a readout struct — but the sim core is still
the unstarted foundation.
