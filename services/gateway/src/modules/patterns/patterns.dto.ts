import { z } from "zod";

import { MarketFeedService } from "../realtime/market-feed.service";

const REGIME_LABELS = ["Trending", "Ranging", "Expansion", "HighVolChoppy"] as const;
const PATTERN_KINDS = ["double_top", "double_bottom"] as const;

export const ListPatternsQueryDto = z.object({
  symbol: z.enum(MarketFeedService.SYMBOLS as [string, ...string[]]).optional(),
  regime: z.enum(REGIME_LABELS).optional(),
});
export type ListPatternsQueryDto = z.infer<typeof ListPatternsQueryDto>;

export const PatternPriorQueryDto = z.object({
  kind: z.enum(PATTERN_KINDS),
  symbol: z.enum(MarketFeedService.SYMBOLS as [string, ...string[]]),
  regime: z.enum(REGIME_LABELS),
  since_ts: z.coerce.number().int().optional(),
});
export type PatternPriorQueryDto = z.infer<typeof PatternPriorQueryDto>;
