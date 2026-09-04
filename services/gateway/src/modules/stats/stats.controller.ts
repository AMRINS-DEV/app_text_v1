import { Controller, Get, UseGuards } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { StatsService } from "./stats.service";

@Controller("api/stats")
@UseGuards(JwtAuthGuard)
export class StatsController {
  constructor(private readonly stats: StatsService) {}

  @Get("overview")
  overview() {
    return this.stats.overview();
  }
}
