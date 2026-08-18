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
}

/**
 * The simulation canvas and everything drawn over it.
 *
 * The canvas itself is behind SimSurface, so the WebGL2 renderer replaces the
 * placeholder without touching this file.
 */
/** Tools that place something on the tile grid, and so want to see it. */
const BUILD_TOOLS = new Set<Tool>(['draw', 'erase', 'spawner']);

export function createViewport(surface: SimSurface, actions: ViewportActions): Component {
  const grid = el('div', { class: 'tile-grid' });
  const root = el('div', { class: 'viewport' }, [surface.canvas, grid]);

  let camera = { x: 0, y: 0, zoom: 4 };
  let spaceHeld = false;
  let dragging: 'pan' | 'paint' | null = null;
  let lastPointer: Vec2 | null = null;
  /** Where the pointer is in client space, so world coords can be recomputed when the
   *  camera moves under a stationary cursor — pressing `F` is exactly that case. */
  let pointerClient: Vec2 | null = null;
  let strokeStart: Vec2 | null = null;
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

  // Panning is a modifier on the pointer, so the key state lives with the pointer
  // handlers rather than in the store — nothing else in the HUD reacts to it.
  window.addEventListener('keydown', (event) => {
    if (event.code === 'Space') spaceHeld = true;
    if (event.key === 'Shift') straight = true;
  });
  window.addEventListener('keyup', (event) => {
    if (event.code === 'Space') spaceHeld = false;
    if (event.key === 'Shift') straight = false;
  });
  window.addEventListener('blur', () => {
    spaceHeld = false;
    straight = false;
  });

  root.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
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
      actions.press(world);
    }
    root.setPointerCapture(event.pointerId);
  });

  root.addEventListener('pointermove', (event) => {
    pointerClient = { x: event.clientX, y: event.clientY };
    const world = toWorld(event);
    actions.moveCursor(world);

    if (dragging === 'pan' && lastPointer) {
      // No inertia, no easing — the sim is a workbench.
      actions.panBy(
        -(event.clientX - lastPointer.x) / camera.zoom,
        -(event.clientY - lastPointer.y) / camera.zoom,
      );
    } else if (dragging === 'paint' && strokeStart) {
      actions.paint(strokeStart, world, straight);
    }
    lastPointer = { x: event.clientX, y: event.clientY };
  });

  const endDrag = (event: PointerEvent): void => {
    dragging = null;
    strokeStart = null;
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
