import { z } from "zod";

/**
 * Mirrors `crates/domain/src/signal.rs::Signal` and
 * `packages/proto/signal.proto` — keep all three in sync. This is the shape
 * the dashboard's signals page (§12) and gateway's SignalsModule (§11.1)
 * validate against at the TS boundary.
 */
export const DirectionSchema = z.enum(["Long", "Short", "Flat"]);
export const RegimeTagSchema = z.enum(["Trending", "Ranging", "Expansion", "HighVolChoppy"]);

export const SignalSchema = z.object({
  id: z.string(),
  source: z.union([
    z.object({ agent: z.string() }),
    z.object({ model: z.string() }),
    z.object({ rule: z.string() }),
  ]),
  symbolId: z.number().int().nonnegative(),
  direction: DirectionSchema,
  probability: z.number().min(0).max(1),
  confidence: z.number().min(0).max(1),
  expectedR: z.number(),
  horizonMs: z.number().int().nonnegative(),
  ttlNs: z.number().int().nonnegative(),
  regime: RegimeTagSchema,
  featuresHash: z.number().int().nonnegative(),
  evidenceRef: z.string().uuid().optional(),
});

export type Signal = z.infer<typeof SignalSchema>;
