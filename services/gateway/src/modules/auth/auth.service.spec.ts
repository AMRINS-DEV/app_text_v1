import * as jwt from "jsonwebtoken";
import { Test } from "@nestjs/testing";

import { AuthService } from "./auth.service";
import { JWT_SECRET, TokenService } from "./token.service";
import { generateTotp } from "./totp";
import { UsersStore } from "./users.store";

async function buildAuthService(): Promise<{ auth: AuthService; users: UsersStore }> {
  const moduleRef = await Test.createTestingModule({
    providers: [AuthService, TokenService, UsersStore, { provide: JWT_SECRET, useValue: "test-secret" }],
  }).compile();
  return { auth: moduleRef.get(AuthService), users: moduleRef.get(UsersStore) };
}

function totpFor(users: UsersStore, username: string): string {
  const user = users.findByUsername(username);
  if (!user) throw new Error("seed user missing");
  return generateTotp(user.totpSecret);
}

describe("AuthService", () => {
  it("rejects a wrong password", async () => {
    const { auth } = await buildAuthService();
    expect(() => auth.login("owner", "wrong")).toThrow();
  });

  it("issues a preauth token on correct password, then tokens on correct TOTP", async () => {
    const { auth, users } = await buildAuthService();
    const { preAuthToken } = auth.login("owner", "owner-dev-password");
    const code = totpFor(users, "owner");
    const pair = auth.verifyTotpAndIssueTokens(preAuthToken, code);
    expect(pair.accessToken).toEqual(expect.any(String));
    expect(pair.refreshToken).toEqual(expect.any(String));
  });

  it("rejects a wrong TOTP code even with a valid preauth token", async () => {
    const { auth } = await buildAuthService();
    const { preAuthToken } = auth.login("owner", "owner-dev-password");
    expect(() => auth.verifyTotpAndIssueTokens(preAuthToken, "000000")).toThrow();
  });

  it("rejects an access token used where a preauth token is required", async () => {
    const { auth, users } = await buildAuthService();
    const { preAuthToken } = auth.login("owner", "owner-dev-password");
    const pair = auth.verifyTotpAndIssueTokens(preAuthToken, totpFor(users, "owner"));
    expect(() => auth.verifyTotpAndIssueTokens(pair.accessToken, "000000")).toThrow();
  });

  it("rotates the refresh token: the old one cannot be reused", async () => {
    const { auth, users } = await buildAuthService();
    const { preAuthToken } = auth.login("owner", "owner-dev-password");
    const first = auth.verifyTotpAndIssueTokens(preAuthToken, totpFor(users, "owner"));
    const second = auth.refresh(first.refreshToken);
    expect(second.accessToken).not.toEqual(first.accessToken);
    expect(() => auth.refresh(first.refreshToken)).toThrow();
  });

  it("issues a step-up token only with a fresh correct TOTP code", async () => {
    const { auth, users } = await buildAuthService();
    const user = users.findByUsername("trader");
    if (!user) throw new Error("seed user missing");
    expect(() => auth.stepUp(user.id, "000000")).toThrow();
    const { stepUpToken } = auth.stepUp(user.id, totpFor(users, "trader"));
    expect(stepUpToken).toEqual(expect.any(String));
  });
});

describe("TokenService", () => {
  async function buildTokenService(): Promise<TokenService> {
    const moduleRef = await Test.createTestingModule({
      providers: [TokenService, { provide: JWT_SECRET, useValue: "test-secret" }],
    }).compile();
    return moduleRef.get(TokenService);
  }

  it("rejects a token of the wrong type", async () => {
    const tokens = await buildTokenService();
    const { token } = tokens.sign("user-1", "trader" as never, "access");
    expect(() => tokens.verify(token, "refresh")).toThrow();
  });

  it("rejects a token signed with a different secret", async () => {
    const tokens = await buildTokenService();
    const forged = jwt.sign({ sub: "user-1", role: "owner", typ: "access", jti: "x" }, "different-secret");
    expect(() => tokens.verify(forged, "access")).toThrow();
  });
});
