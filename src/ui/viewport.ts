import { TILE_CELLS } from '../constants';
import { el } from './dom';
import type { Component } from './component';
import type { GameState, Tool, Vec2 } from '../state/types';
import type { SimSurface } from '../sim/surface';

export interface ViewportActions {
  /** Cursor moved, in world cells. */
  moveCursor(world: Vec2): void;
  /** Space-drag panning, in world cells. */
  panBy(dx: number, dy: number): void;
  /** A press in the world, before any drag. What it does depends on the tool. */
  press(world: Vec2): void;
  /** alt+click: pick the material under the cursor. Needs the sim to sample. */
  pickMaterialAt(world: Vec2): void;
  /** A drag with the draw or erase tool. `straight` is shift being held. */
  paint(from: Vec2, to: Vec2, straight: boolean): void;
  /** A drag with the vault tool: the region it covered becomes storage. */
  designateVault(from: Vec2, to: Vec2): void;
  /** A drag with the belt or filter tool: the line it covered becomes conveyor. */
  paintBelt(from: Vec2, to: Vec2): void;
  /** What the pointer is doing to the world, for the ghost. */
  setStroke(anchor: Vec2 | null, painting: boolean): void;
}

/**
 * The simulation canvas and everything drawn over it.
 *
 * The canvas itself is behind SimSurface, so the WebGL2 renderer replaces the
 * placeholder without touching this file.
 */
/** Tools that place something on the tile grid, and so want to see it. */
const BUILD_TOOLS = new Set<Tool>([
  'draw',
  'erase',
  'spawner',
  'press',
  'burner',
  'compactor',
  'heater',
  'vault',
  'belt',
  'filter',
]);

export function createViewport(surface: SimSurface, actions: ViewportActions): Component {
  const grid = el('div', { class: 'tile-grid' });
  const root = el('div', { class: 'viewport' }, [surface.canvas, grid]);

  let camera = { x: 0, y: 0, zoom: 4 };
  /** Mirrored from the store so the pointer handlers know what a drag means. */
  let selectedTool: Tool = 'draw';
  let spaceHeld = false;
  let dragging: 'pan' | 'paint' | null = null;
  let lastPointer: Vec2 | null = null;
  /** Where the pointer is in client space, so world coords can be recomputed when the
   *  camera moves under a stationary cursor — pressing `F` is exactly that case. */
  let pointerClient: Vec2 | null = null;
  /** The last world position the pointer reported, for resuming a stroke cleanly. */
  let lastWorld: Vec2 | null = null;
  /** Where the stroke began. Only the shift-constrained line anchors to it. */
  let strokeStart: Vec2 | null = null;
  /** The last position painted to. A free stroke follows the path the pointer took, so
   *  each move draws from here — anchoring every move to the stroke's origin instead
   *  sweeps a fan of lines and fills the region between them. */
  let strokePrevious: Vec2 | null = null;
  /** Whether this drag is marking out a region rather than painting one. */
  let marking = false;
  /** Whether this drag is laying out a line of belt or filter tiles. */
  let laying = false;
  let straight = false;

  /** Screen pixels -> world cells, about the viewport centre. */
  const worldAt = (clientX: number, clientY: number): Vec2 => {
    const rect = root.getBoundingClientRect();
    return {
      x: Math.floor(camera.x + (clientX - rect.left - rect.width / 2) / camera.zoom),
      y: Math.floor(camera.y + (clientY - rect.top - rect.height / 2) / camera.zoom),
    };
  };

  const toWorld = (event: PointerEvent): Vec2 => worldAt(event.clientX, event.clientY);

  const observer = new ResizeObserver(() => {
    surface.resize(root.clientWidth, root.clientHeight, window.devicePixelRatio || 1);
  });
  observer.observe(root);

  /** Tells the store what the pointer is doing to the world, so the ghost can draw it.
   *  Called on transitions only: the cursor is already in state and the anchor does not
   *  move within a stroke, so there is nothing to update per frame. */
  const reportStroke = (): void => {
    // A vault, or a belt/filter line, is dragged out and lands on release, so it
    // previews the same way a constrained stroke does.
    const previewing = dragging === 'paint' && (straight || marking || laying);
    actions.setStroke(previewing ? strokeStart : null, dragging === 'paint');
  };

  // Panning is a modifier on the pointer, so the key state lives with the pointer
  // handlers rather than in the store — nothing else in the HUD reacts to it.
  window.addEventListener('keydown', (event) => {
    if (event.code === 'Space') spaceHeld = true;
    if (event.key === 'Shift' && !straight) {
      // Taking shift mid-stroke stops painting and starts previewing.
      straight = true;
      reportStroke();
    }
  });
  window.addEventListener('keyup', (event) => {
    if (event.code === 'Space') spaceHeld = false;
    if (event.key === 'Shift' && straight) {
      straight = false;
      // Resume freehand from where the pointer is now, not from where the constrained
      // stroke began — otherwise letting go of shift paints back across the world.
      strokePrevious = lastWorld ?? strokePrevious;
      reportStroke();
    }
  });
  window.addEventListener('blur', () => {
    spaceHeld = false;
    straight = false;
    // Losing focus mid-stroke would otherwise leave the preview hanging over the world.
    reportStroke();
  });

  root.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
    // Overlays — drawers, the inspector, the notice — are children of the viewport so
    // they can sit over the canvas, which means their presses bubble here. A press on
    // the HUD is not a press on the world, and before there was anything to click in a
    // drawer this quietly placed machines underneath one.
    if (event.target !== surface.canvas) return;
    const world = toWorld(event);
    lastPointer = { x: event.clientX, y: event.clientY };
    pointerClient = lastPointer;

    if (spaceHeld) {
      dragging = 'pan';
    } else if (event.altKey) {
      actions.pickMaterialAt(world);
    } else {
      dragging = 'paint';
      strokeStart = world;
      strokePrevious = world;
      marking = selectedTool === 'vault';
      laying = selectedTool === 'belt' || selectedTool === 'filter';
      reportStroke();
      // A stroke that previews — constrained, marking out a vault, or laying a belt
      // line — commits on release, so pressing must not act. Everything else acts on
      // the press.
      if (!straight && !marking && !laying) actions.press(world);
    }
    root.setPointerCapture(event.pointerId);
  });

  root.addEventListener('pointermove', (event) => {
    pointerClient = { x: event.clientX, y: event.clientY };
    const world = toWorld(event);
    lastWorld = world;
    actions.moveCursor(world);

    if (dragging === 'pan' && lastPointer) {
      // No inertia, no easing — the sim is a workbench.
      actions.panBy(
        -(event.clientX - lastPointer.x) / camera.zoom,
        -(event.clientY - lastPointer.y) / camera.zoom,
      );
    } else if (dragging === 'paint' && strokePrevious && !straight && !marking && !laying) {
      // Free strokes paint as they go, along the path the pointer took. A constrained
      // one only previews here — it commits once, on release.
      actions.paint(strokePrevious, world, false);
      strokePrevious = world;
    }
    lastPointer = { x: event.clientX, y: event.clientY };
  });

  const endDrag = (event: PointerEvent): void => {
    // All three of these have been a preview until now. This is the commit.
    if (dragging === 'paint' && strokeStart && lastWorld) {
      if (marking) actions.designateVault(strokeStart, lastWorld);
      else if (laying) actions.paintBelt(strokeStart, lastWorld);
      else if (straight) actions.paint(strokeStart, lastWorld, true);
    }
    marking = false;
    laying = false;
    dragging = null;
    strokeStart = null;
    strokePrevious = null;
    reportStroke();
    lastPointer = null;
    if (root.hasPointerCapture(event.pointerId)) root.releasePointerCapture(event.pointerId);
  };
  root.addEventListener('pointerup', endDrag);
  root.addEventListener('pointercancel', endDrag);

  let frame = requestAnimationFrame(function draw() {
    frame = requestAnimationFrame(draw);
    surface.render(camera);
  });
  window.addEventListener('beforeunload', () => cancelAnimationFrame(frame));

  return {
    root,

    update(state: GameState) {
      const moved = camera !== state.ui.camera;
      camera = state.ui.camera;
      selectedTool = state.ui.selectedTool;

      // The cursor sits still while the camera moves under it, so the world position
      // it names has changed. Re-emit outside this update to avoid re-entering the
      // store mid-notify.
      if (moved && pointerClient && !dragging) {
        const at = pointerClient;
        queueMicrotask(() => actions.moveCursor(worldAt(at.x, at.y)));
      }

      // Building is grid-based, so the grid shows itself while a build tool is held
      // rather than being something the player has to know to switch on. `G` still
      // toggles it for the tools that are not building anything.
      const showGrid = state.ui.showTileGrid || BUILD_TOOLS.has(state.ui.selectedTool);
      grid.hidden = !showGrid;
      if (showGrid) {
        // One tile at the current zoom. The offset follows the camera so the grid
        // stays pinned to the world rather than to the screen.
        const pitch = TILE_CELLS * camera.zoom;
        const offsetX = ((-camera.x * camera.zoom + root.clientWidth / 2) % pitch + pitch) % pitch;
        const offsetY = ((-camera.y * camera.zoom + root.clientHeight / 2) % pitch + pitch) % pitch;
        grid.style.backgroundSize = `${pitch}px ${pitch}px`;
        grid.style.backgroundPosition = `${offsetX}px ${offsetY}px`;
      }
    },
  };
}
