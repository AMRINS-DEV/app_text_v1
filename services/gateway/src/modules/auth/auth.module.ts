import { Module } from "@nestjs/common";
import { AuthController } from "./auth.controller";

/**
 * JWT (short-lived) + refresh, TOTP 2FA, step-up auth for trading actions
 * (§11.1, §13). Token issuance/verification is Phase 4 scope — this fixes
 * the controller surface (§11.2: POST /api/auth/login) so RBAC guards in
 * other modules have something to depend on.
 */
@Module({
  controllers: [AuthController],
})
export class AuthModule {}
