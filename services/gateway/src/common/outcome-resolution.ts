/**
 * §12.3's CONFIRMED/FAILED/TIMEOUT verdict, computed from real subsequent
 * OHLC price action — the same triple-barrier shape as
 * `agents_graph.outcomes.resolve_pattern_outcome` (Python, §7 knowledge
 * layer) and `agents_models.labeling.triple_barrier` (§8.2), reimplemented
 * here since the gateway (TypeScript) and the agent layer (Python) have no
 * live bridge in this sandbox — the same cross-language duplication as the
 * Rust domain types and their TS Zod mirrors in `packages/schemas`.
 */
export type Verdict = "confirmed" | "failed" | "timeout";

export interface OutcomeResolution {
  verdict: Verdict;
  barsToResolution: number | null;
  mfe: number;
  mae: number;
  rMultiple: number;
  movePips: number;
  moveAtr: number;
  direction: "Long" | "Short";
}

export function resolvePatternOutcome(
  direction: "Long" | "Short",
  entryPrice: number,
  targetPrice: number,
  invalidationPrice: number,
  subsequentHighs: readonly number[],
  subsequentLows: readonly number[],
  pipSize: number,
  atr: number,
): OutcomeResolution {
  const isLong = direction === "Long";
  const riskDistance = Math.abs(entryPrice - invalidationPrice);
  if (riskDistance <= 0) throw new Error("invalidationPrice must differ from entryPrice");

  let maxFavorable = 0;
  let maxAdverse = 0;

  for (let i = 0; i < subsequentHighs.length; i++) {
    const high = subsequentHighs[i];
    const low = subsequentLows[i];
    let hitTarget: boolean;
    let hitInvalidation: boolean;

    if (isLong) {
      maxFavorable = Math.max(maxFavorable, high - entryPrice);
      maxAdverse = Math.max(maxAdverse, entryPrice - low);
      hitTarget = high >= targetPrice;
      hitInvalidation = low <= invalidationPrice;
    } else {
      maxFavorable = Math.max(maxFavorable, entryPrice - low);
      maxAdverse = Math.max(maxAdverse, high - entryPrice);
      hitTarget = low <= targetPrice;
      hitInvalidation = high >= invalidationPrice;
    }

    // A bar touching both barriers resolves toward the adverse outcome —
    // OHLC bars can't disambiguate intrabar sequencing.
    if (hitInvalidation) {
      return buildResolution("failed", i, entryPrice, invalidationPrice, isLong, maxFavorable, maxAdverse, riskDistance, pipSize, atr);
    }
    if (hitTarget) {
      return buildResolution("confirmed", i, entryPrice, targetPrice, isLong, maxFavorable, maxAdverse, riskDistance, pipSize, atr);
    }
  }

  return {
    verdict: "timeout",
    barsToResolution: null,
    mfe: maxFavorable,
    mae: maxAdverse,
    rMultiple: 0,
    movePips: 0,
    moveAtr: 0,
    direction,
  };
}

function buildResolution(
  verdict: Verdict,
  barsToResolution: number,
  entryPrice: number,
  exitPrice: number,
  isLong: boolean,
  mfe: number,
  mae: number,
  riskDistance: number,
  pipSize: number,
  atr: number,
): OutcomeResolution {
  const signedMove = isLong ? exitPrice - entryPrice : entryPrice - exitPrice;
  return {
    verdict,
    barsToResolution,
    mfe,
    mae,
    rMultiple: signedMove / riskDistance,
    movePips: signedMove / pipSize,
    moveAtr: atr > 0 ? signedMove / atr : 0,
    direction: isLong ? "Long" : "Short",
  };
}

export interface FixedHorizonMove {
  movePips: number;
  moveAtr: number;
  direction: "Long" | "Short" | "Flat";
  directionHit: boolean;
}

/** §7.2 query #2's fixed-horizon measurement — no barriers, just "what
 * actually happened by this horizon." Shared by the news module too. */
export function resolveFixedHorizonMove(
  priceAtEvent: number,
  priceAtHorizon: number,
  expectedDirection: "Long" | "Short" | "Flat",
  pipSize: number,
  atr: number,
): FixedHorizonMove {
  const signedMove = priceAtHorizon - priceAtEvent;
  const realizedDirection: "Long" | "Short" | "Flat" = signedMove > 0 ? "Long" : signedMove < 0 ? "Short" : "Flat";
  return {
    movePips: signedMove / pipSize,
    moveAtr: atr > 0 ? signedMove / atr : 0,
    direction: realizedDirection,
    directionHit: realizedDirection === expectedDirection,
  };
}
