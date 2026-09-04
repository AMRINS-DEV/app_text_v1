import { Module } from "@nestjs/common";

import { AuthModule } from "../auth/auth.module";
import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { RolesGuard } from "../../common/roles.guard";
import { StepUpGuard } from "../../common/step-up.guard";

/**
 * Roles: owner | trader | analyst | viewer; per-action guards (§11.1, §13).
 * `JwtAuthGuard` verifies the access token; `RolesGuard` reads `@Roles(...)`;
 * `StepUpGuard` reads `@RequireStepUp()`.
 *
 * Nest resolves a class passed to `@UseGuards()` against the *hosting
 * controller's own module* injectables, not against providers merely
 * exported by an imported module further down the graph — so importing
 * this module alone does not make `@UseGuards(JwtAuthGuard, ...)` work in
 * a different module. Each feature module that protects routes (see
 * `TradingModule`) re-declares these three as its own providers, importing
 * `AuthModule` directly for `TokenService`; this module exists as the one
 * place their wiring is documented, and is still usable directly by a
 * module whose controllers don't need `@UseGuards()` at all (e.g. one that
 * only injects `AuthService`/`TokenService` via constructor).
 */
@Module({
  imports: [AuthModule],
  providers: [JwtAuthGuard, RolesGuard, StepUpGuard],
  exports: [JwtAuthGuard, RolesGuard, StepUpGuard],
})
export class RbacModule {}
