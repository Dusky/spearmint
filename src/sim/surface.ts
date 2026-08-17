import type { Camera } from '../state/types';

/**
 * The seam between the HUD and whatever draws the world.
 *
 * The real implementation is a WebGL2 renderer uploading a texture from the typed
 * array the Rust/WASM sim owns (spec §9). The HUD only ever sees this interface, so
 * swapping the placeholder for it touches no UI code.
 */
export interface SimSurface {
  readonly canvas: HTMLCanvasElement;
  /** CSS pixel size of the viewport, plus the device pixel ratio to render at. */
  resize(width: number, height: number, devicePixelRatio: number): void;
  render(camera: Camera): void;
  dispose(): void;
}
