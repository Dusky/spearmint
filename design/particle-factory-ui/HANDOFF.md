# Handoff: Particle Factory — in-game UI shell

## Overview

Interface design for **Particle Factory**, an incremental factory game built on a side-on
falling-sand particle simulation (see the project's own design & technical spec — this
document covers only the UI layer).

Scope of this handoff: the **in-game HUD** that frames the live simulation viewport, plus
three supporting panels (capability/upgrade list, blueprint library, and the chrome's own
style rules). It does not cover the title screen, settings, onboarding, or leaderboards —
those are not designed yet.

The design deliberately has **no styling theme**. The chrome is neutral and quiet because
the simulation is the only thing on screen worth looking at. The design work is in the
information architecture, not the decoration.

## About the design files

The files in this bundle are **design references created in HTML** — a prototype showing
intended look and behaviour, not production code to copy. The task is to **recreate this
design in the game's real environment**. For this project that means the TypeScript/WebGL2
client described in the spec: the HUD should be DOM overlaid on the WebGL canvas (or the
UI framework the client settles on), not HTML-in-an-iframe.

`Particle Factory UI.dc.html` will not run standalone outside its authoring environment —
read it as a specification of markup structure and exact style values. `support.js` is the
authoring runtime and has nothing to do with the game; ignore it except to open the file.

The embedded falling-sand simulation in the prototype (in the file's logic class) is a
**visual stand-in only** — a quick JS cellular automaton written so the chrome could be
judged against real content. It is not a reference implementation. The real sim is
Rust→WASM, fixed-point, deterministic, per the spec. Do not port this JS.

## Fidelity

**High fidelity.** Colours, type, spacing, and copy are final-intent. Recreate pixel-for-pixel
using the client's own styling approach. Two things are explicitly unresolved and flagged
in "Open questions" below.

---

## Screens / Views

### 1. In-game HUD (the only screen)

**Purpose:** the player draws machines into a live particle sim, watches them run, and
diagnoses why they underperform.

**Overall frame:** `1160px` wide in the prototype, but this is a **fluid, full-viewport
layout** in the real game — the viewport (canvas) flexes and everything else is fixed-size.
Background `#1E1E1D`, `1px` border `#33322F`, radius `5px` (the border/radius exist only
because the prototype sits on a page; in-game the HUD is edge-to-edge and both are dropped).

Vertical structure, top to bottom:

| Region | Height | Notes |
|---|---|---|
| Top bar | `54px` | economy readouts |
| Body | flex | tool column (`150px`) + viewport (flex) |
| Status bar | `36px` | modifier-key hints + cursor coords |

---

#### 1.1 Top bar

`display:flex; align-items:center; height:54px; padding:0 18px; gap:32px;`
bottom border `1px solid #2B2A28`.

Left group — **gold**: `display:flex; align-items:baseline; gap:10px`
- Label `GOLD` — Archivo 11px/600, letter-spacing `.09em`, `#7C7A73`
- Value `4 812` — IBM Plex Mono 23px/500, letter-spacing `-.02em`, `#EAE9E5`
  (thin-space thousands separator, not a comma)
- Rate `+38/s` — IBM Plex Mono 13px, `#8FAF7E`

Divider: `1px × 24px`, `#33322F`.

Second group — **spawners**: `gap:11px`
- Label `SPAWNERS` — same label style as GOLD
- Pip row: five `9×18px` rects, radius `1px`, gap `4px`. Owned `#C9C7C0`, unowned `#33322F`.
- Text `3 / 5` — IBM Plex Mono 13px, `#98968F`

Right group (after `flex:1` spacer): IBM Plex Mono 12px, `#75736C`, gap `24px`
- `tick 1 284 905` — increments live
- `seed 4f2a11`
- `60 Hz` in `#98968F`, preceded by a `6px` circle `#8FAF7E`

**Note:** spawner count is the game's only hard input cap, which is why it sits in the top
bar next to gold rather than in a menu.

---

#### 1.2 Tool column — `150px`, right border `1px solid #2B2A28`, `padding:12px 0`

Section label `TOOLS` — Archivo 10px/600, letter-spacing `.11em`, `#6B6963`,
`padding:0 14px 9px`.

Tool rows — `display:flex; align-items:center; gap:11px; padding:8px 14px;`
`border-left:2px solid transparent`.
- **Selected:** background `#2F2E2B`, left border `#EAE9E5`, label Archivo 14px/600 `#EAE9E5`, hotkey `#8B8982`
- **Default:** label Archivo 14px/400 `#C9C7C0`, hotkey IBM Plex Mono 11px `#6B6963`
- **Hover:** background `#252523`

Rows and their glyphs (all glyphs are plain CSS boxes — no icon font, no SVG):

| Row | Key | Glyph |
|---|---|---|
| Draw | 1 | `16×16` filled square, radius 2 |
| Erase | 2 | `16×16` outlined square, `1px #6B6963` |
| Belt | 3 | `16×9` filled bar, radius 1 |
| Spawner | 4 | `16×16` outlined circle |
| Teleport | 5 | `14×14` square rotated 45° |
| Blueprint | 6 | `16×16` dashed square |

Divider `1px #2B2A28`, margin `14px`.

Section `MATERIAL` (same label style), then a swatch row — four `26×26` swatches,
radius `3px`, gap `6px`:
`#5C5850` wall · `#C3BDAC` insulator · `#A8442A` heater · `#3E6B8A` cooler.
Selected swatch: `outline:2px solid #EAE9E5; outline-offset:1px`.
Beneath: `wall · 9×9 snap` — IBM Plex Mono 11px `#75736C` (9×9 is the tile size in
particle cells).

Bottom of column, pushed down with `margin-top:auto`, top border `1px #2B2A28`,
`padding:14px 14px 2px`, Archivo 12px/1.45 `#6B6963`:

> Machines must start themselves. Nothing is primed on load.

**This line is load-bearing.** Because live particle state is never persisted, every
machine must be self-starting — the spec requires that this be communicated rather than
discovered. It's permanent chrome, not a dismissible tip.

---

#### 1.3 Viewport

The simulation canvas. Background `#070806`. Fills all remaining space.
`image-rendering: pixelated`.

Overlays, all absolutely positioned over the canvas:

**Tile grid** (toggleable, off by default): two `linear-gradient` hairlines at
`rgba(234,233,229,.07)`, `background-size:36px 36px` (= one 9×9 tile at 4× zoom).
`pointer-events:none`.

**Selection marquee:** `1px solid rgba(234,233,229,.85)`, radius `2px`, `pointer-events:none`.
Label tab flush to its top-left, sitting *above* the box (`top:-26px`): background `#EAE9E5`,
text `#1B1B1A`, Archivo 11px/600, `padding:3px 8px`, radius `2px 2px 0 0`.
Copy pattern: `Washer 02 · 24×20 tiles` — player-given name, then footprint in tiles.

**Inspector panel** — top-right, `14px` inset, width `272px`.
Background `rgba(30,30,29,.96)`, border `1px solid #3A3936`, radius `4px`.
Internal dividers are `1px #2B2A28` full-bleed rules.

1. Header (`padding:11px 13px 9px`): machine name — Archivo 14px/600 `#EAE9E5`;
   right-aligned status `running` — IBM Plex Mono 11px `#8FAF7E`.
2. Yield block (`padding:0 13px 12px`):
   - Row: `YIELD` label (11px/600, `.09em`, `#7C7A73`) and `best seen 82%` (Mono 12px `#75736C`)
   - Big number `68%` — Mono 30px/500 `#EAE9E5`, the `%` at 15px `#98968F`
   - Bar: `4px` tall, radius 2, track `#33322F`, fill `#C9C7C0` at the current percentage,
     plus a `1px #6B6963` tick at the best-observed percentage, overhanging `3px` top and bottom
3. Measurements block (`padding:11px 13px`), each row `display:flex; justify-content:space-between; padding:4px 0`,
   IBM Plex Mono 12px — key `#7C7A73`, value `#EAE9E5`:
   `temperature 384 K` · `contact area 1 940 cells` · `residence 41 ticks` · `mixing 0.62`
   **The out-of-range value turns `#E0A23D`** — that is the only highlight; there is no icon or badge.
4. Diagnosis (`padding:11px 13px`), Archivo 13px/1.45 `#C9C7C0`:
   > Residence is short — water leaves before it wets the sand. Narrow the outlet or raise the weir.

**This panel is the single opinionated design decision.** Because reaction yield varies
with layout rather than fixed ratios, the interface's job is to explain *why* a machine
underperforms. It reports physical conditions and names the bottleneck in a sentence — it
never reports an abstracted throughput number, which the spec forbids.

**Problem notice** — bottom-left, `14px` inset.
`display:flex; align-items:center; gap:10px;` background `rgba(30,30,29,.96)`,
border `1px solid #57482C` with `border-left:2px solid #E0A23D`, radius `3px`,
`padding:8px 12px`.
- Message — Archivo 13px `#EAE9E5`: `Belt 07 is buried — slag backed up 340 cells`
- Action hint — Mono 11px `#8B8982`, `padding-left:4px`, `border-left:1px solid #3A3936`: `F — jump there`

Clogging is emergent physics, not a rule, so this notice is **informational, not an alarm**:
no red, no modal, no sound implied. It states the belt, the material, and the depth, offers
a jump key, and gets out of the way.

---

#### 1.4 Status bar

`height:36px; padding:0 18px; gap:26px;` top border `1px #2B2A28`.
IBM Plex Mono 11px `#6B6963`. Left: `shift — straight line`, `space — pan`,
`alt — pick material`. Right (after spacer): world coords `x 1 284 · y −406`
(true minus sign, U+2212).

---

### 2. Supporting panels

Three panels below the HUD in the prototype. **In-game these are overlays/drawers, not a
second row** — the prototype lays them out flat so all states are visible at once. Shared
shell: background `#1E1E1D`, border `1px solid #33322F`, radius `5px`.
Title Archivo 14px/600 `#EAE9E5` at `padding:13px 15px 4px`; optional sub-line Archivo 12px
`#75736C`; then a full-bleed `1px #2B2A28` rule.

#### 2.1 Capability (`376px`)
Sub-line: *Gold buys reach. Raw material builds machines.*

Rows: `display:flex; align-items:center; gap:13px; padding:11px 0`,
separated by `1px solid #262523` (last row has none).
- Title — Archivo 14px/500 `#EAE9E5`
- Description — Archivo 12px/1.4 `#75736C`
- Price — IBM Plex Mono 14px `#EAE9E5`, right-aligned

Content:
| Title | Description | Price |
|---|---|---|
| Teleport range 4 → 5 cells | Straight line only. | 2 400 |
| Blueprint slot 3 → 4 | Pasted machines obey local conditions. | 1 150 |
| Spawner 4 | The only thing that raises your ceiling. | 18 000 |

**Unaffordable rows are the whole row at `opacity:.45`** — not greyed text, not disabled
styling. Spawner 4 is shown in that state.

#### 2.2 Blueprints (`340px`)
Header right side: `3 / 4` — Mono 12px `#75736C`. Sub-line: *Thumbnails are the real thing, not an icon.*

`display:grid; grid-template-columns:1fr 1fr; gap:10px; padding:12px 15px 15px`.

Each card: a `70px`-tall thumbnail — background `#070806`, `1px solid #2B2A28`, radius `3px`,
`overflow:hidden` — then name (Archivo 13px/500 `#EAE9E5`, `margin-top:6px`) and meta
(Mono 11px `#75736C`).

Thumbnails must be **rendered from the blueprint's actual construct data** at the sim's own
palette (walls `#5C5850`, sand `#C8A24A`, water `#3E7FA8`, wet sand `#8A7A52`) — the prototype
fakes them with positioned rects. Meta format: `24×20 · 82%` (footprint · best yield observed);
for a machine with no output, footprint only.

Cards: `Washer 02 / 24×20 · 82%`, `Settler A / 18×26 · 71%`, `Slag dump / 40×12`.
Empty slot: `70px` box, `1px dashed #3A3936`, radius `3px`, centred Archivo 12px `#6B6963` `empty`.

#### 2.3 Rules of the chrome (fills remaining width)
Documentation panel — a swatch row (five `44px` chips, radius `3px`, gap `6px`) over the
prose in **Design tokens** below. Not part of the game; delete on implementation.

---

## Interactions & behaviour

Nothing here is animated in the prototype; all of it is intended.

- **Tool selection** — number keys `1`–`6` and click. Selection is instant, no transition.
- **Material selection** — click a swatch; `alt`+click in the world picks the material under the cursor.
- **Drawing** — click-drag paints at the tile snap (9×9 cells). `shift` constrains to a straight line.
- **Panning** — hold `space` and drag. No inertia, no easing — the sim is a workbench.
- **Selection** — click a construct to select; the inspector opens top-right and updates every frame.
- **Inspector values update live** at the sim's tick rate, throttled to a readable cadence
  (~4 Hz is enough; per-frame is unreadable and wasteful). Values must not reflow — hence monospace.
- **Diagnosis sentence** is derived from whichever measurement is furthest out of range;
  when everything is in range it reads as a plain statement of what limits the machine.
- **Problem notice** appears when a belt clogs or buries; `F` pans the camera to it.
  Dismiss on click. Only one is shown at a time — the most recent.
- **Hover** on tool rows only (`#252523`). Panels and cards have no hover state.
- **No transitions anywhere** in the current design. If any are added, cap them at ~120ms
  and never animate anything overlaying the sim.

## State

- `selectedTool` — draw | erase | belt | spawner | teleport | blueprint
- `selectedMaterial` — wall | insulator | heater | cooler
- `selection` — the selected construct's id, or null (drives the inspector + marquee)
- `showTileGrid` — boolean, off by default
- `camera` — world x/y, zoom (status-bar coords, panning)
- Economy, read from server-authoritative state: `gold`, `goldRate`, `spawnersOwned`,
  `spawnersMax`, `blueprintSlots`, purchased upgrades
- Sim-derived, read-only per selection: `temperature`, `contactArea`, `residence`, `mixing`,
  `yieldCurrent`, `yieldBest`
- `notices[]` — active clog/burial events

`yieldBest` ("best seen") is a **per-blueprint high-water mark**, so it needs storing with
the blueprint, not the machine instance.

## Design tokens

**Colour** — four greys and two signal colours. Colour never decorates: if something is
coloured, it changed.

| Token | Hex | Use |
|---|---|---|
| Sim void | `#070806` | canvas background; also blueprint thumbnails |
| Surface | `#1E1E1D` | HUD and panel background |
| Surface raised | `#2F2E2B` | selected tool row |
| Surface hover | `#252523` | tool row hover |
| Border | `#33322F` | panel outer border, divider pips |
| Border subtle | `#2B2A28` | internal rules |
| Border panel (overlay) | `#3A3936` | inspector/notice borders |
| Text primary | `#EAE9E5` | values, selected labels |
| Text secondary | `#C9C7C0` | unselected labels, prose |
| Text muted | `#98968F` | units, secondary numerics |
| Text faint | `#7C7A73` / `#75736C` / `#6B6963` | field labels, hints, hotkeys |
| Signal — attention | `#E0A23D` | out-of-range value, problem notice accent |
| Signal — running | `#8FAF7E` | gold rate, running status, tick indicator |

Material swatches: wall `#5C5850`, insulator `#C3BDAC`, heater `#A8442A`, cooler `#3E6B8A`.
Sim particle palette (stand-in): sand `#C8A24A`, water `#3E7FA8`, wet sand `#8A7A52`,
wall `#5C5850` — each with ±10 per-channel per-cell variance from a static noise table.

**Type** — mono for data, grotesk for labels.
- **Archivo** 400/500/600 — labels, tool names, prose, headings. Sentence case except the
  all-caps 10–11px field labels. No shouting.
- **IBM Plex Mono** 400/500 — every number, hotkey, and coordinate. Non-negotiable:
  readouts must not jitter while they tick.

Scale in use: 10, 11, 12, 13, 14, 23, 30px. Caps labels use letter-spacing `.09em`–`.11em`;
large numerics use `-.02em`.

**Spacing** — 4 / 6 / 9 / 10 / 11 / 13 / 14 / 15 / 18 / 24 / 32px. Panel padding is
`13px 15px`; HUD bars pad `0 18px`.

**Radius** — 1 (pips), 2 (small glyphs, marquee), 3 (swatches, notices, thumbnails),
4 (inspector), 5 (panels). In-game the HUD frame itself has none.

**Elevation** — none. No shadows, no gradients, no textures, no blur. Panels separate with
one hairline and one background step. Overlays over the canvas use `rgba(30,30,29,.96)`
rather than a backdrop filter.

## Assets

None. No images, no icon font, no SVG. Every glyph is a CSS box (square, circle, bar,
rotated square, dashed square). Fonts are Google Fonts: Archivo and IBM Plex Mono —
self-host both in the real client.

## Files

- `Particle Factory UI.dc.html` — the design prototype. The markup is the layout spec;
  the trailing script class contains only the stand-in sand sim and the tick counter.
- `support.js` — authoring runtime. Not part of the design. Ignore.

## Open questions for the design side

Flag these back rather than inventing answers:

1. **Tool column orientation.** It is a left column here. If the sim reads best as a wide
   landscape strip, a bottom bar may be better — this affects the whole frame.
2. **Where teleporters and blueprints live** once they become the progression spine. They
   are tool-column entries today, which likely under-serves them.
3. **Multiple simultaneous problem notices.** Only one is designed. A queue, a stack, or a
   counter are all plausible and none is chosen.
4. **Zoom levels.** The tile grid is drawn at one zoom. Its behaviour when zoomed out past
   tile legibility is undesigned.
