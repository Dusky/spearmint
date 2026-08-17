/* SCAFFOLDING — delete when the WebGL2 renderer and the sim land.
 *
 * A stand-in so the chrome can be judged against moving content. It is deliberately
 * NOT a particle simulation: it hashes a fixed function of world position into the
 * sim's palette and pans that with the camera, plus a slow shimmer so the viewport
 * is visibly live. There is no state, no gravity, and nothing to port.
 *
 * The real sim is Rust→WASM, fixed-point and deterministic (spec §3.1). */

import type { Camera } from '../state/types';
import type { SimSurface } from './surface';

const PALETTE: readonly (readonly [number, number, number])[] = [
  [0x07, 0x08, 0x06], // void
  [0x5c, 0x58, 0x50], // wall
  [0xc8, 0xa2, 0x4a], // sand
  [0x3e, 0x7f, 0xa8], // water
  [0x8a, 0x7a, 0x52], // wet sand
];

/** Deterministic integer hash — same cell, same value, every frame. */
function hash(x: number, y: number): number {
  let h = (x * 0x1f1f1f1f) ^ (y * 0x2545f491);
  h = Math.imul(h ^ (h >>> 15), 0x2c1b3c6d);
  h = Math.imul(h ^ (h >>> 12), 0x297a2d39);
  return (h ^ (h >>> 15)) >>> 0;
}

export function createPlaceholderSurface(): SimSurface {
  const canvas = document.createElement('canvas');
  canvas.className = 'viewport__canvas';

  const context = canvas.getContext('2d', { alpha: false });
  if (!context) throw new Error('2d context unavailable');
  context.imageSmoothingEnabled = false;

  // Cell-resolution buffer, blown up to the canvas with nearest-neighbour sampling.
  const buffer = document.createElement('canvas');
  const bufferContext = buffer.getContext('2d', { alpha: false });
  if (!bufferContext) throw new Error('2d context unavailable');

  let image: ImageData | null = null;
  let pixels: Uint32Array | null = null;

  const surface: SimSurface = {
    canvas,

    resize(width, height, devicePixelRatio) {
      canvas.width = Math.max(1, Math.round(width * devicePixelRatio));
      canvas.height = Math.max(1, Math.round(height * devicePixelRatio));
      context.imageSmoothingEnabled = false;
      image = null;
    },

    render(camera: Camera) {
      const scale = Math.max(1, camera.zoom);
      const cellsWide = Math.max(1, Math.ceil(canvas.width / scale));
      const cellsHigh = Math.max(1, Math.ceil(canvas.height / scale));

      if (!image || image.width !== cellsWide || image.height !== cellsHigh) {
        buffer.width = cellsWide;
        buffer.height = cellsHigh;
        image = bufferContext.createImageData(cellsWide, cellsHigh);
        pixels = new Uint32Array(image.data.buffer);
      }
      if (!pixels) return;

      const originX = Math.round(camera.x - cellsWide / 2);
      const originY = Math.round(camera.y - cellsHigh / 2);
      const shimmer = Math.floor(performance.now() / 120);

      for (let y = 0; y < cellsHigh; y++) {
        const worldY = originY + y;
        for (let x = 0; x < cellsWide; x++) {
          const worldX = originX + x;
          const noise = hash(worldX, worldY);

          // Two scales of block noise, mixed, so deposits come out as irregular
          // clumps rather than slabs. Coverage is kept low: the world is mostly
          // void, and the chrome has to hold up against sparse content as well as
          // busy content.
          const coarse = hash(worldX >> 5, worldY >> 4) % 100;
          const fine = hash(worldX >> 2, worldY >> 2) % 100;
          const density = coarse * 0.65 + fine * 0.35;

          let kind = 0;
          if (density > 74) {
            const band = hash(worldX >> 6, worldY >> 6) % 100;
            if (band > 74) kind = 1;
            else if (band > 46) kind = 2;
            else if (band > 20) kind = 3;
            else kind = 4;
            if (noise % 100 < 22) kind = 0; // ragged edges
          }

          // A few cells wink, so the viewport is visibly alive without simulating.
          if (kind !== 0 && (noise ^ shimmer) % 997 === 0) kind = 0;

          const colour = PALETTE[kind] ?? PALETTE[0];
          if (!colour) continue;
          // Variance is per particle. The void stays flat — speckling it would read
          // as material that isn't there.
          const variance = kind === 0 ? 0 : (noise % 21) - 10; // ±10 per channel
          const r = Math.min(255, Math.max(0, colour[0] + variance));
          const g = Math.min(255, Math.max(0, colour[1] + variance));
          const b = Math.min(255, Math.max(0, colour[2] + variance));
          pixels[y * cellsWide + x] = (255 << 24) | (b << 16) | (g << 8) | r;
        }
      }

      bufferContext.putImageData(image, 0, 0);
      context.drawImage(buffer, 0, 0, cellsWide, cellsHigh, 0, 0, canvas.width, canvas.height);
    },

    dispose() {
      image = null;
      pixels = null;
    },
  };

  return surface;
}
