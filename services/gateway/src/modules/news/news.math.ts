import type { NewsEventRecord } from "./news-history";

/** §7.2 query #2's news impact persistence, reimplemented over the
 * gateway's own synthetic news history — same quarter-bucketed
 * aggregation as `agents_graph.queries.news_impact_persistence`. */
export interface NewsImpactPeriod {
  quarter: string;
  n: number;
  avgImpact: number;
  directionHitRate: number;
}

function quarterKey(ts: number): string {
  const date = new Date(ts);
  const quarter = Math.floor(date.getUTCMonth() / 3) + 1;
  return `${date.getUTCFullYear()}Q${quarter}`;
}

export function newsImpactStability(
  history: readonly NewsEventRecord[],
  eventType: string,
  symbol: string,
  horizonMin: number,
): NewsImpactPeriod[] {
  const byQuarter = new Map<string, { impact: number; hit: boolean }[]>();
  for (const record of history) {
    if (record.eventType !== eventType || record.symbol !== symbol || record.horizonMin !== horizonMin) continue;
    const key = quarterKey(record.ts);
    const bucket = byQuarter.get(key) ?? [];
    bucket.push({ impact: Math.abs(record.move.moveAtr), hit: record.move.directionHit });
    byQuarter.set(key, bucket);
  }

  return [...byQuarter.entries()]
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([quarter, samples]) => ({
      quarter,
      n: samples.length,
      avgImpact: samples.reduce((sum, sample) => sum + sample.impact, 0) / samples.length,
      directionHitRate: samples.filter((sample) => sample.hit).length / samples.length,
    }));
}
