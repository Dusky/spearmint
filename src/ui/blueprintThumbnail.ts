import type { Blueprint } from '../state/types';

/**
 * Renders a blueprint thumbnail from its actual construct data, at the sim's own
 * palette. Thumbnails are the real thing, not an icon — so this reads the same cell
 * array the sim would hand over, and nothing here is drawn by hand.
 */

const PALETTE: Record<number, readonly [number, number, number]> = {
  0: [0x07, 0x08, 0x06], // empty — the sim void
  1: [0x5c, 0x58, 0x50], // wall
  2: [0xc8, 0xa2, 0x4a], // sand
  3: [0x3e, 0x7f, 0xa8], // water
  4: [0x8a, 0x7a, 0x52], // wet sand
};

/** Stable per-cell variance, so a thumbnail looks identical every time it is drawn. */
function variance(index: number): number {
  let h = Math.imul(index ^ 0x9e3779b9, 0x85ebca6b);
  h ^= h >>> 13;
  return ((h >>> 0) % 21) - 10; // ±10 per channel, per cell
}

export function renderBlueprintThumbnail(blueprint: Blueprint): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.width = blueprint.cellWidth;
  canvas.height = blueprint.cellHeight;
  canvas.setAttribute('role', 'img');
  canvas.setAttribute('aria-label', blueprint.name);

  const context = canvas.getContext('2d');
  if (!context) return canvas;

  const image = context.createImageData(blueprint.cellWidth, blueprint.cellHeight);
  const pixels = new Uint32Array(image.data.buffer);

  for (let i = 0; i < pixels.length; i++) {
    const kind = blueprint.cells[i] ?? 0;
    const colour = PALETTE[kind] ?? PALETTE[0];
    if (!colour) continue;
    const shift = kind === 0 ? 0 : variance(i);
    const r = Math.min(255, Math.max(0, colour[0] + shift));
    const g = Math.min(255, Math.max(0, colour[1] + shift));
    const b = Math.min(255, Math.max(0, colour[2] + shift));
    pixels[i] = (255 << 24) | (b << 16) | (g << 8) | r;
  }

  context.putImageData(image, 0, 0);
  return canvas;
}
