import { z } from "zod";

export const RiskProfileDto = z.object({
  riskPerTradePct: z.number().gt(0).lte(5),
  fractionalKellyCap: z.number().gt(0).lte(1),
  maxDailyDrawdownPct: z.number().gt(0).lte(50),
  maxTotalDrawdownPct: z.number().gt(0).lte(100),
});

export const UpdateSettingsDto = z.object({
  riskProfile: RiskProfileDto.optional(),
  allowedPairs: z.array(z.string().min(1)).min(1).optional(),
  defaultMode: z.enum(["live", "paper"]).optional(),
  modelRouting: z.record(z.string(), z.string()).optional(),
});
export type UpdateSettingsDto = z.infer<typeof UpdateSettingsDto>;
