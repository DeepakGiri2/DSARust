// Numeric helpers with Rust's semantics where they differ from JavaScript's.
// The renderers are line-for-line ports, and a port that silently changes how
// NaN or a negative float cast behaves is a port that draws something else.

/** `f32::clamp`: a NaN input stays NaN (as `Math.min`/`Math.max` also keep it). */
export function clamp(x: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, x))
}

export function clamp01(t: number): number {
  return clamp(t, 0, 1)
}

/** `f64::max`: a NaN argument loses to the other one (IEEE maxNum), unlike `Math.max`. */
export function fmax(a: number, b: number): number {
  if (Number.isNaN(a)) return b
  if (Number.isNaN(b)) return a
  return Math.max(a, b)
}

/** `x as usize` for a float: truncates toward zero, saturates at 0, and NaN becomes 0. */
export function asUsize(x: number): number {
  return Number.isNaN(x) ? 0 : Math.max(0, Math.trunc(x))
}

/** `x as u8` for a float: truncates, saturates to 0..=255, NaN becomes 0. */
export function asU8(x: number): number {
  return Number.isNaN(x) ? 0 : Math.min(255, Math.max(0, Math.trunc(x)))
}

/** Rust's `f32::round`: halves go away from zero (`Math.round` sends -2.5 to -2). */
export function roundHalfAway(x: number): number {
  return x < 0 ? -Math.round(-x) : Math.round(x)
}
