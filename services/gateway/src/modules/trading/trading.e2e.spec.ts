import { Test } from "@nestjs/testing";
import type { INestApplication } from "@nestjs/common";
import { WsAdapter } from "@nestjs/platform-ws";
import request from "supertest";

import { AuthModule } from "../auth/auth.module";
import { AuthService } from "../auth/auth.service";
import { generateTotp } from "../auth/totp";
import { UsersStore } from "../auth/users.store";
import { TradingModule } from "./trading.module";

/** Proves the real HTTP wiring — guard ordering, `@Roles`/`@RequireStepUp`
 * metadata scoping, and the full login → TOTP → step-up flow — rather than
 * just each guard's `canActivate` in isolation. */
describe("TradingController (e2e)", () => {
  let app: INestApplication;

  async function loginAs(username: string): Promise<string> {
    const auth = app.get(AuthService);
    const users = app.get(UsersStore);
    const user = users.findByUsername(username);
    if (!user) throw new Error("seed user missing");
    const { preAuthToken } = auth.login(username, `${username}-dev-password`);
    const { accessToken } = auth.verifyTotpAndIssueTokens(preAuthToken, generateTotp(user.totpSecret));
    return accessToken;
  }

  async function stepUpFor(username: string): Promise<string> {
    const auth = app.get(AuthService);
    const users = app.get(UsersStore);
    const user = users.findByUsername(username);
    if (!user) throw new Error("seed user missing");
    return auth.stepUp(user.id, generateTotp(user.totpSecret)).stepUpToken;
  }

  beforeAll(async () => {
    const moduleRef = await Test.createTestingModule({
      imports: [AuthModule, TradingModule],
    }).compile();
    app = moduleRef.createNestApplication();
    app.useWebSocketAdapter(new WsAdapter(app));
    await app.init();
  });

  afterAll(async () => {
    await app.close();
  });

  it("rejects an unauthenticated request", async () => {
    await request(app.getHttpServer()).get("/api/trading/positions").expect(401);
  });

  it("allows a viewer to read positions", async () => {
    const token = await loginAs("viewer");
    await request(app.getHttpServer())
      .get("/api/trading/positions")
      .set("Authorization", `Bearer ${token}`)
      .expect(200);
  });

  it("forbids a viewer from firing the kill switch", async () => {
    const token = await loginAs("viewer");
    await request(app.getHttpServer())
      .post("/api/trading/kill-switch")
      .set("Authorization", `Bearer ${token}`)
      .expect(403);
  });

  it("lets a trader fire the kill switch with no step-up token at all", async () => {
    const token = await loginAs("trader");
    await request(app.getHttpServer())
      .post("/api/trading/kill-switch")
      .set("Authorization", `Bearer ${token}`)
      .expect(201);
  });

  it("refuses a mode change without a step-up token", async () => {
    const token = await loginAs("trader");
    await request(app.getHttpServer())
      .post("/api/trading/mode")
      .set("Authorization", `Bearer ${token}`)
      .send({ mode: "paper" })
      .expect(403);
  });

  it("accepts a mode change once the kill switch is reset and a valid step-up token is presented", async () => {
    const token = await loginAs("trader");
    const stepUp = await stepUpFor("trader");
    await request(app.getHttpServer())
      .post("/api/trading/kill-switch/reset")
      .set("Authorization", `Bearer ${token}`)
      .set("x-step-up-token", stepUp)
      .expect(201);

    const secondStepUp = await stepUpFor("trader");
    await request(app.getHttpServer())
      .post("/api/trading/mode")
      .set("Authorization", `Bearer ${token}`)
      .set("x-step-up-token", secondStepUp)
      .send({ mode: "paper" })
      .expect(201);
  });

  it("rejects a mode value the DTO doesn't allow", async () => {
    const token = await loginAs("trader");
    const stepUp = await stepUpFor("trader");
    await request(app.getHttpServer())
      .post("/api/trading/mode")
      .set("Authorization", `Bearer ${token}`)
      .set("x-step-up-token", stepUp)
      .send({ mode: "halted" })
      .expect(400);
  });
});
