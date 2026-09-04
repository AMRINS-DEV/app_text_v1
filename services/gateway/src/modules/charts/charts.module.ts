import { Module } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { AuthModule } from "../auth/auth.module";
import { ChartsController } from "./charts.controller";
import { ChartsService } from "./charts.service";

/** Bars, indicators, downsampled series (§11.1). */
@Module({
  imports: [AuthModule],
  controllers: [ChartsController],
  providers: [ChartsService, JwtAuthGuard],
})
export class ChartsModule {}
