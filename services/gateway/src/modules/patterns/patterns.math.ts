import type { PatternInstanceRecord } from "./pattern-history";

/** §7.2 query #1's conditional reliability, reimplemented over the
 * gateway's own synthetic pattern history — same aggregation as
 * `agents_graph.queries.conditional_reliability`, just over an array
 * instead of a graph traversal (no relationships to walk on this side of
 * the language boundary). */
export interface ConditionalReliability {
  n: number;
  hitRate: number;
  avgR: number;
  medianR: number;
}

function percentileCont(values: readonly number[], p: number): number {
  const sorted = [...values].sort((a, b) => a - b);
  if (sorted.length === 1) return sorted[0];
  const index = p * (sorted.length - 1);
  const lower = Math.floor(index);
  const upper = Math.min(lower + 1, sorted.length - 1);
  const fraction = index - lower;
  return sorted[lower] + (sorted[upper] - sorted[lower]) * fraction;
}

export function conditionalReliability(
  history: readonly PatternInstanceRecord[],
  kind: string,
  symbol: string,
  regime: string,
  sinceTs = 0,
): ConditionalReliability {
  // Every resolved instance counts toward n/hitRate, timeout included —
  // a timeout is "not confirmed," the same as a failure, matching
  // agents_graph's own RESOLVED_AS semantics (confirmed = verdict ==
  // CONFIRMED, nothing else).
  const matches = history.filter(
    (record) => record.kind === kind && record.symbol === symbol && record.regime === regime && record.tsStart >= sinceTs,
  );
  if (matches.length === 0) return { n: 0, hitRate: 0, avgR: 0, medianR: 0 };

  const n = matches.length;
  const hitRate = matches.filter((record) => record.resolution.verdict === "confirmed").length / n;
  const rMultiples = matches.map((record) => record.resolution.rMultiple);
  const avgR = rMultiples.reduce((sum, r) => sum + r, 0) / n;
  return { n, hitRate, avgR, medianR: percentileCont(rMultiples, 0.5) };
}
