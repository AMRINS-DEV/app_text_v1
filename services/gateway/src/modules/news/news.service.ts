import { Injectable } from "@nestjs/common";

import { generateSyntheticNewsHistory, NewsEventRecord } from "./news-history";
import { newsImpactStability, NewsImpactPeriod } from "./news.math";

/** §11.1/§12.4: "feed, impact analysis, graph queries." Real ingestion
 * (the news-agent's LLM triage, §10.1) lives in the Python agent layer
 * with no live bridge to this gateway in this sandbox — see
 * `news-history.ts`'s doc comment for what's synthetic vs. real here. */
@Injectable()
export class NewsService {
  private readonly history: NewsEventRecord[] = generateSyntheticNewsHistory();

  timeline(symbol?: string): NewsEventRecord[] {
    return this.history.filter((record) => symbol === undefined || record.symbol === symbol);
  }

  impactStability(eventType: string, symbol: string, horizonMin: number): NewsImpactPeriod[] {
    return newsImpactStability(this.history, eventType, symbol, horizonMin);
  }
}
