import type { Camera } from '../state/types';
import type { Sim } from './wasm';
import type { SimSurface } from './surface';

/**
 * Draws the running simulation.
 *
 * A 2D canvas rather than the WebGL2 renderer spec 9 calls for. The vertical slice is
 * testing whether building a machine is satisfying, not rendering technology, and this
 * path is already proven at this scale. `SimSurface` is the seam that makes the swap
 * free when it matters.
 *
 * One canvas pixel is one cell; the page scales it up with nearest-neighbour sampling,
 * so zooming costs nothing.
 */
export function createWasmSurface(sim: Sim): SimSurface {
  const canvas = document.createElement('canvas');
  canvas.className = 'viewport__canvas';

  const context = canvas.getContext('2d', { alpha: false });
  if (!context) throw new Error('2d context unavailable');
  context.imageSmoothingEnabled = false;

  let cellsWide = 1;
  let cellsHigh = 1;

  return {
    canvas,

    resize(width, height, _devicePixelRatio) {
      // Sized in cells, not device pixels: the sim has no sub-cell detail to render, so
      // a higher pixel ratio would only cost fill rate.
      cellsWide = Math.max(1, Math.ceil(width / 4));
      cellsHigh = Math.max(1, Math.ceil(height / 4));
      canvas.width = cellsWide;
      canvas.height = cellsHigh;
      context.imageSmoothingEnabled = false;
    },

    render(camera: Camera) {
      const originX = Math.round(camera.x - cellsWide / 2);
      const originY = Math.round(camera.y - cellsHigh / 2);

      const pixels = sim.render(originX, originY, cellsWide, cellsHigh);
      // Must be consumed before the next call into wasm — the buffer is reused.
      context.putImageData(new ImageData(pixels, cellsWide, cellsHigh), 0, 0);
    },

    dispose() {
      // Nothing to release: the frame buffer lives in wasm memory.
    },
  };
}
