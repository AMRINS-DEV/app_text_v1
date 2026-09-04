import { Module } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { RolesGuard } from "../../common/roles.guard";
import { AuthModule } from "../auth/auth.module";
import { SettingsController } from "./settings.controller";
import { SettingsService } from "./settings.service";

/** Risk profiles, allowed pairs, modes, agent config, model routing (§11.1). */
@Module({
  imports: [AuthModule],
  controllers: [SettingsController],
  providers: [SettingsService, JwtAuthGuard, RolesGuard],
  exports: [SettingsService],
})
export class SettingsModule {}
