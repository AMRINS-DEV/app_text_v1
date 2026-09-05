import { z } from "zod";

import { MarketFeedService } from "../realtime/market-feed.service";

export const ListNewsQueryDto = z.object({
  symbol: z.enum(MarketFeedService.SYMBOLS as [string, ...string[]]).optional(),
});
export type ListNewsQueryDto = z.infer<typeof ListNewsQueryDto>;

export const NewsImpactStabilityQueryDto = z.object({
  event_type: z.string().min(1),
  symbol: z.enum(MarketFeedService.SYMBOLS as [string, ...string[]]),
  horizon_min: z.coerce.number().int().positive(),
});
export type NewsImpactStabilityQueryDto = z.infer<typeof NewsImpactStabilityQueryDto>;
