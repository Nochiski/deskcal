// Dev-only render counters. `PERF` is a compile-time constant false in production builds
// (import.meta.env.DEV folds to false), so countRender() becomes an empty function there.
export const PERF: boolean =
  import.meta.env.DEV && typeof location !== "undefined" && new URLSearchParams(location.search).has("perf");

const counts: Record<string, number> = {};

/** Call at the top of a component's render. */
export function countRender(name: string): void {
  if (PERF) counts[name] = (counts[name] ?? 0) + 1;
}

export function perfSnapshot(): Record<string, number> {
  return { ...counts };
}

/** Difference of two snapshots, only non-zero entries, sorted by name. */
export function perfDiff(a: Record<string, number>, b: Record<string, number>): string {
  const keys = Array.from(new Set([...Object.keys(a), ...Object.keys(b)])).sort();
  return keys
    .map((k) => [k, (b[k] ?? 0) - (a[k] ?? 0)] as const)
    .filter(([, n]) => n !== 0)
    .map(([k, n]) => `${k}=${n}`)
    .join(" ");
}
