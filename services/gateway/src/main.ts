import "reflect-metadata";
import { NestFactory } from "@nestjs/core";
import { WsAdapter } from "@nestjs/platform-ws";
import { AppModule } from "./app.module";

async function bootstrap() {
  const app = await NestFactory.create(AppModule);
  app.enableCors();
  // §11.3: WS /ws/stream. `ws`-backed adapter for now; MessagePack framing
  // and per-topic coalescing/backpressure are Phase 4 scope.
  app.useWebSocketAdapter(new WsAdapter(app));
  const port = process.env.PORT ? Number(process.env.PORT) : 4000;
  await app.listen(port);
  // eslint-disable-next-line no-console
  console.log(`TradeOS gateway listening on :${port}`);
}

bootstrap();
