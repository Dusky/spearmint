/** Number formatting for the HUD.
 *
 *  Thousands are separated with a thin space (U+2009), not a comma, and negatives use
 *  a true minus sign (U+2212), not a hyphen. Both are in the handoff. */

const THIN_SPACE = ' ';
const MINUS = '−';

/** 4812 -> "4 812" (thin space). */
export function formatInteger(value: number): string {
  const rounded = Math.round(value);
  const digits = Math.abs(rounded).toString();
  let grouped = '';
  for (let i = 0; i < digits.length; i++) {
    if (i > 0 && (digits.length - i) % 3 === 0) grouped += THIN_SPACE;
    grouped += digits[i];
  }
  return rounded < 0 ? MINUS + grouped : grouped;
}

/** 0.68 -> "68". The unit is rendered separately, at its own size. */
export function formatPercent(fraction: number): string {
  return Math.round(fraction * 100).toString();
}

/** 0.62 -> "0.62". Fixed decimals so the value cannot change width as it ticks. */
export function formatDecimal(value: number, places: number): string {
  const text = value.toFixed(places);
  return text.startsWith('-') ? MINUS + text.slice(1) : text;
}

/** A world position for the status bar: "x 1 284 · y −406". */
export function formatCoordinates(x: number, y: number): string {
  return `x ${formatInteger(x)} · y ${formatInteger(y)}`;
}

/** A footprint for blueprint meta and the marquee label: "24×20". */
export function formatFootprint(width: number, height: number): string {
  return `${width}×${height}`;
}
