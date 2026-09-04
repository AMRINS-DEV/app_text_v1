import { Module } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { RolesGuard } from "../../common/roles.guard";
import { StepUpGuard } from "../../common/step-up.guard";
import { AuthModule } from "../auth/auth.module";
import { RealtimeModule } from "../realtime/realtime.module";
import { AuditLogService } from "./audit-log.service";
import { InMemoryCoreClient } from "./in-memory-core-client";
import { CORE_CLIENT } from "./trading.constants";
import { TradingController } from "./trading.controller";
import { TradingService } from "./trading.service";

/**
 * §11.1: "gRPC → core: positions, orders, mode switch, kill switch." See
 * `trading.types.ts`'s `CoreClient` doc comment for why this binds to
 * `InMemoryCoreClient` instead of a real gRPC client in this environment.
 *
 * `JwtAuthGuard`/`RolesGuard`/`StepUpGuard` are re-declared as providers
 * here (not just imported via a shared module) because Nest resolves a
 * class passed to `@UseGuards()` against the *hosting module's own*
 * injectables — a provider merely exported by an imported module isn't
 * enough. `AuthModule` being imported (here, in `RealtimeModule`, and
 * anywhere else) always resolves to the same singleton `TokenService`, so
 * this repetition costs nothing behaviorally.
 */
@Module({
  imports: [AuthModule, RealtimeModule],
  controllers: [TradingController],
  providers: [
    TradingService,
    AuditLogService,
    JwtAuthGuard,
    RolesGuard,
    StepUpGuard,
    { provide: CORE_CLIENT, useClass: InMemoryCoreClient },
  ],
  exports: [TradingService, AuditLogService],
})
export class TradingModule {}
