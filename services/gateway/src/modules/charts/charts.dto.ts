import { z } from "zod";

import { MarketFeedService } from "../realtime/market-feed.service";

export const BarsQueryDto = z.object({
  sym: z.enum(MarketFeedService.SYMBOLS as [string, ...string[]]),
  tf: z.enum(["5s", "1m", "5m", "1h"]),
  from: z.coerce.number().int(),
  to: z.coerce.number().int(),
  max_points: z.coerce.number().int().positive().max(5_000).optional(),
});
export type BarsQueryDto = z.infer<typeof BarsQueryDto>;
