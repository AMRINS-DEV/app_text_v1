import { Body, Controller, Get, NotImplementedException, Param, Post } from "@nestjs/common";

/** §11.2 key API surface (abridged) for the trading module. */
@Controller("api/trading")
export class TradingController {
  @Post("mode")
  setMode(@Body() _body: { mode: string }): never {
    // Step-up auth required (§11.2) — enforced once AuthModule/RbacModule land.
    throw new NotImplementedException("trading.mode is Phase 4 scope (needs gRPC client to tradeos-core)");
  }

  @Post("kill-switch")
  killSwitch(): never {
    // Immediate flatten+halt, single atomic operation, audit-logged (§9.5, §11.2).
    throw new NotImplementedException("trading.kill-switch is Phase 4 scope (needs gRPC client to tradeos-core)");
  }

  @Get("positions")
  positions(): never {
    throw new NotImplementedException("trading.positions is Phase 4 scope (needs gRPC client to tradeos-core)");
  }

  @Post("positions/:id/close")
  closePosition(@Param("id") _id: string, @Body() _body: { fraction?: number }): never {
    throw new NotImplementedException("trading.positions.close is Phase 4 scope (needs gRPC client to tradeos-core)");
  }
}
