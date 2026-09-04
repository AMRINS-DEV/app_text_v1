import { z } from "zod";

export const SetModeDto = z.object({
  mode: z.enum(["live", "paper"]),
});
export type SetModeDto = z.infer<typeof SetModeDto>;

export const ClosePositionDto = z.object({
  fraction: z.number().gt(0).lte(1).optional(),
});
export type ClosePositionDto = z.infer<typeof ClosePositionDto>;
