import { Controller, Get, Query, UseGuards } from "@nestjs/common";

import { JwtAuthGuard } from "../../common/jwt-auth.guard";
import { ZodBody } from "../../common/zod-body.pipe";
import { ListPatternsQueryDto, PatternPriorQueryDto } from "./patterns.dto";
import { PatternsService } from "./patterns.service";

@Controller("api/patterns")
@UseGuards(JwtAuthGuard)
export class PatternsController {
  constructor(private readonly patterns: PatternsService) {}

  @Get()
  list(@Query(new ZodBody(ListPatternsQueryDto)) query: ListPatternsQueryDto) {
    return this.patterns.list(query.symbol, query.regime);
  }

  @Get("prior")
  prior(@Query(new ZodBody(PatternPriorQueryDto)) query: PatternPriorQueryDto) {
    return this.patterns.historicalPrior(query.kind, query.symbol, query.regime, query.since_ts);
  }
}
