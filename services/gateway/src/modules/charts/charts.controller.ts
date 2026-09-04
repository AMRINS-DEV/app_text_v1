import { Controller, Get, Query, UseGuards } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { ZodBody } from "../../common/zod-body.pipe";
import { BarsQueryDto } from "./charts.dto";
import { ChartsService } from "./charts.service";

@Controller("api/charts")
@UseGuards(JwtAuthGuard)
export class ChartsController {
  constructor(private readonly charts: ChartsService) {}

  @Get("bars")
  bars(@Query(new ZodBody(BarsQueryDto)) query: BarsQueryDto) {
    return this.charts.bars(query);
  }
}
