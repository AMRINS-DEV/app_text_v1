import { Test } from "@nestjs/testing";
import { NestFactory } from "@nestjs/core";
import { WsAdapter } from "@nestjs/platform-ws";
import { decode } from "@msgpack/msgpack";
import { WebSocket } from "ws";

import { AuthModule } from "../auth/auth.module";
import { TokenService } from "../auth/token.service";
import { Role } from "../../common/roles";
import { RealtimeModule } from "./realtime.module";

/**
 * A real end-to-end check of the WS transport: boots an actual Nest HTTP+WS
 * server on an ephemeral port and connects with a real `ws` client, rather
 * than unit-testing `RealtimeGateway`'s methods in isolation — the framing,
 * auth-on-connect, and RBAC-on-subscribe behavior all live in how the
 * adapter wires `handleConnection`/`@SubscribeMessage` together, which a
 * pure unit test would just be re-asserting past.
 */
describe("RealtimeGateway (e2e)", () => {
  let app: import("@nestjs/common").INestApplication;
  let port: number;
  let tokens: TokenService;

  beforeAll(async () => {
    const moduleRef = await Test.createTestingModule({
      imports: [AuthModule, RealtimeModule],
    }).compile();

    app = moduleRef.createNestApplication();
    app.useWebSocketAdapter(new WsAdapter(app));
    await app.listen(0);
    port = (app.getHttpServer().address() as { port: number }).port;
    tokens = moduleRef.get(TokenService);
  });

  afterAll(async () => {
    await app.close();
  });

  function connect(role: Role): Promise<WebSocket> {
    const { token } = tokens.sign("user-1", role, "access");
    return new Promise((resolve, reject) => {
      const ws = new WebSocket(`ws://127.0.0.1:${port}/ws/stream?token=${token}`);
      ws.once("open", () => resolve(ws));
      ws.once("error", reject);
    });
  }

  function waitForFrame(ws: WebSocket, timeoutMs = 500): Promise<{ topic: string; ts: number; payload: unknown }> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("timed out waiting for a frame")), timeoutMs);
      ws.once("message", (data: Buffer) => {
        clearTimeout(timer);
        resolve(decode(data) as { topic: string; ts: number; payload: unknown });
      });
    });
  }

  it("rejects a connection with no token", async () => {
    const closed = new Promise<number>((resolve) => {
      const ws = new WebSocket(`ws://127.0.0.1:${port}/ws/stream`);
      ws.once("close", (code: number) => resolve(code));
    });
    expect(await closed).toBe(4001);
  });

  it("rejects a connection with an invalid token", async () => {
    const closed = new Promise<number>((resolve) => {
      const ws = new WebSocket(`ws://127.0.0.1:${port}/ws/stream?token=garbage`);
      ws.once("close", (code: number) => resolve(code));
    });
    expect(await closed).toBe(4001);
  });

  it("delivers a coalesced MessagePack tick frame after subscribing", async () => {
    const ws = await connect(Role.Trader);
    ws.send(JSON.stringify({ event: "subscribe", data: { topic: "ticks:EURUSD" } }));
    const frame = await waitForFrame(ws);
    expect(frame.topic).toBe("ticks:EURUSD");
    expect(Array.isArray(frame.payload)).toBe(true);
    const batch = frame.payload as Array<{ symbol: string; bid: number; ask: number }>;
    expect(batch.length).toBeGreaterThan(0);
    expect(batch[0].symbol).toBe("EURUSD");
    expect(batch[0].ask).toBeGreaterThan(batch[0].bid);
    ws.close();
  });

  it("stops delivering frames for a topic after unsubscribing", async () => {
    const ws = await connect(Role.Trader);
    ws.send(JSON.stringify({ event: "subscribe", data: { topic: "ticks:GBPUSD" } }));
    await waitForFrame(ws);
    ws.send(JSON.stringify({ event: "unsubscribe", data: { topic: "ticks:GBPUSD" } }));
    // Drain any frame already in flight from before the unsubscribe took effect.
    await new Promise((resolve) => setTimeout(resolve, 100));
    await expect(waitForFrame(ws, 300)).rejects.toThrow("timed out");
    ws.close();
  });

  it("conflates a low-frequency topic to its latest snapshot only", async () => {
    const ws = await connect(Role.Trader);
    ws.send(JSON.stringify({ event: "subscribe", data: { topic: "agent_status" } }));
    const frame = await waitForFrame(ws, 4_000);
    expect(frame.topic).toBe("agent_status");
    expect(Array.isArray(frame.payload)).toBe(false);
    ws.close();
  });

  it("denies a role not allowed on a topic (viewer -> agent_status)", async () => {
    const ws = await connect(Role.Viewer);
    ws.send(JSON.stringify({ event: "subscribe", data: { topic: "agent_status" } }));
    await expect(waitForFrame(ws, 400)).rejects.toThrow("timed out");
    ws.close();
  });
});
