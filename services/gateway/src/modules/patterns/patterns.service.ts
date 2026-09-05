import { Injectable } from "@nestjs/common";

import { ConditionalReliability, conditionalReliability } from "./patterns.math";
import { generateSyntheticPatternHistory, PatternInstanceRecord } from "./pattern-history";

/** §11.1/§12.3: "job trigger -> agent, results, verification stats." Real
 * detection (the pattern-agent's geometry, §10.1) lives in the Python
 * agent layer with no live bridge to this gateway in this sandbox — see
 * `pattern-history.ts`'s doc comment for what's synthetic vs. real here. */
@Injectable()
export class PatternsService {
  private readonly history: PatternInstanceRecord[] = generateSyntheticPatternHistory();

  list(symbol?: string, regime?: string): PatternInstanceRecord[] {
    return this.history.filter(
      (record) => (symbol === undefined || record.symbol === symbol) && (regime === undefined || record.regime === regime),
    );
  }

  historicalPrior(kind: string, symbol: string, regime: string, sinceTs = 0): ConditionalReliability {
    return conditionalReliability(this.history, kind, symbol, regime, sinceTs);
  }
}
