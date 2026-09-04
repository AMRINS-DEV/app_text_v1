import { z } from "zod";

/**
 * Mirrors `crates/strategy/src/config.rs::StrategyConfig` (§5.5). The
 * dashboard's settings page edits this shape; the gateway validates it
 * before handing it to the core.
 */
export const StrategyConfigSchema = z.object({
  id: z.string(),
  symbols: z.array(z.string()),
  modes: z.array(z.string()),
  sessions: z.array(z.string()),
  entry: z.object({ requireAll: z.array(z.unknown()) }),
  vetoAny: z.array(z.unknown()).default([]),
  exit: z.object({
    stop: z.unknown(),
    target: z.unknown(),
    trailing: z.unknown().optional(),
    breakeven: z.unknown().optional(),
    quickProfit: z.unknown().optional(),
    timeStop: z.unknown().optional(),
  }),
  sizing: z.object({
    method: z.string(),
    kellyFraction: z.number(),
    riskPerTradePct: z.number(),
    maxConcurrent: z.number().int().positive(),
  }),
});

export type StrategyConfig = z.infer<typeof StrategyConfigSchema>;
