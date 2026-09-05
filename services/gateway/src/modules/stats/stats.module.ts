import { Module } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { AuthModule } from "../auth/auth.module";
import { StatsController } from "./stats.controller";
import { StatsService } from "./stats.service";

/** P&L, equity curve, per-strategy/per-agent expectancy (§11.1). */
@Module({
  imports: [AuthModule],
  controllers: [StatsController],
  providers: [StatsService, JwtAuthGuard],
})
export class StatsModule {}
