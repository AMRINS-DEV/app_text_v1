import { z } from "zod";

export const LoginDto = z.object({
  username: z.string().min(1),
  password: z.string().min(1),
});
export type LoginDto = z.infer<typeof LoginDto>;

export const TotpDto = z.object({
  preAuthToken: z.string().min(1),
  totpCode: z.string().regex(/^\d{6}$/, "TOTP code must be 6 digits"),
});
export type TotpDto = z.infer<typeof TotpDto>;

export const RefreshDto = z.object({
  refreshToken: z.string().min(1),
});
export type RefreshDto = z.infer<typeof RefreshDto>;

export const StepUpDto = z.object({
  totpCode: z.string().regex(/^\d{6}$/, "TOTP code must be 6 digits"),
});
export type StepUpDto = z.infer<typeof StepUpDto>;
