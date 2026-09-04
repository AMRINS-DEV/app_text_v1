import { Module } from "@nestjs/common";

import { AuthController } from "./auth.controller";
import { AuthService } from "./auth.service";
import { JWT_SECRET, TokenService } from "./token.service";
import { UsersStore } from "./users.store";

/**
 * JWT (short-lived) + refresh, TOTP 2FA, step-up auth for trading actions
 * (§11.1, §13). `JWT_SECRET` must be set outside local dev; the fallback
 * below exists only so `just dev`/tests work without extra setup and is not
 * a production credential. Signing uses `jsonwebtoken` directly rather than
 * `@nestjs/jwt` v12, which ships ESM-only and breaks ts-jest's CommonJS
 * transform.
 */
@Module({
  controllers: [AuthController],
  providers: [
    AuthService,
    TokenService,
    UsersStore,
    { provide: JWT_SECRET, useValue: process.env.JWT_SECRET ?? "dev-only-insecure-secret-change-me" },
  ],
  exports: [AuthService, TokenService, UsersStore],
})
export class AuthModule {}
