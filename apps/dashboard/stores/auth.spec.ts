import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useAuthStore } from "./auth";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status, headers: { "Content-Type": "application/json" } });
}

function base64url(input: string): string {
  return Buffer.from(input, "utf8").toString("base64").replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fakeAccessToken(sub: string, role: string): string {
  const header = base64url(JSON.stringify({ alg: "HS256" }));
  const payload = base64url(JSON.stringify({ sub, role, typ: "access", exp: 9_999_999_999, iat: 0, jti: "j" }));
  return `${header}.${payload}.sig`;
}

describe("useAuthStore", () => {
  beforeEach(() => {
    useAuthStore.setState({
      accessToken: null,
      refreshToken: null,
      role: null,
      userId: null,
      pendingPreAuthToken: null,
    });
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("submitPassword stores the pre-auth token from the login response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(jsonResponse(200, { preAuthToken: "pre-123", expiresInSeconds: 120 })),
    );

    await useAuthStore.getState().submitPassword("owner", "owner-dev-password");

    expect(useAuthStore.getState().pendingPreAuthToken).toBe("pre-123");
  });

  it("verifyTotp exchanges the pending pre-auth token for tokens and decodes the role", async () => {
    useAuthStore.setState({ pendingPreAuthToken: "pre-123" });
    const accessToken = fakeAccessToken("user-owner", "owner");
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(jsonResponse(200, { accessToken, refreshToken: "refresh-abc", expiresInSeconds: 300 })),
    );

    await useAuthStore.getState().verifyTotp("123456");

    const state = useAuthStore.getState();
    expect(state.accessToken).toBe(accessToken);
    expect(state.refreshToken).toBe("refresh-abc");
    expect(state.role).toBe("owner");
    expect(state.userId).toBe("user-owner");
    expect(state.pendingPreAuthToken).toBeNull();
  });

  it("verifyTotp without a pending pre-auth token throws rather than calling the API", async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    await expect(useAuthStore.getState().verifyTotp("123456")).rejects.toThrow("submitPassword");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("authorizedFetch attaches the access token and returns the parsed body on success", async () => {
    useAuthStore.setState({ accessToken: "access-1" });
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { ok: true }));
    vi.stubGlobal("fetch", fetchMock);

    const result = await useAuthStore.getState().authorizedFetch<{ ok: boolean }>("/api/settings");

    expect(result).toEqual({ ok: true });
    const [, init] = fetchMock.mock.calls[0];
    expect((init.headers as Headers).get("Authorization")).toBe("Bearer access-1");
  });

  it("authorizedFetch refreshes once and retries on a 401, then succeeds", async () => {
    useAuthStore.setState({ accessToken: "expired", refreshToken: "refresh-1" });
    const newAccessToken = fakeAccessToken("user-1", "trader");
    const fetchMock = vi
      .fn()
      // first call: the expired-token request
      .mockResolvedValueOnce(jsonResponse(401, { message: "expired" }))
      // second call: the refresh call
      .mockResolvedValueOnce(
        jsonResponse(200, { accessToken: newAccessToken, refreshToken: "refresh-2", expiresInSeconds: 300 }),
      )
      // third call: the retried original request
      .mockResolvedValueOnce(jsonResponse(200, { ok: true }));
    vi.stubGlobal("fetch", fetchMock);

    const result = await useAuthStore.getState().authorizedFetch<{ ok: boolean }>("/api/settings");

    expect(result).toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(useAuthStore.getState().accessToken).toBe(newAccessToken);
    const [, thirdInit] = fetchMock.mock.calls[2];
    expect((thirdInit.headers as Headers).get("Authorization")).toBe(`Bearer ${newAccessToken}`);
  });

  it("authorizedFetch does not retry a second time if the refreshed request still 401s", async () => {
    useAuthStore.setState({ accessToken: "expired", refreshToken: "refresh-1" });
    const newAccessToken = fakeAccessToken("user-1", "trader");
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(401, {}))
      .mockResolvedValueOnce(
        jsonResponse(200, { accessToken: newAccessToken, refreshToken: "refresh-2", expiresInSeconds: 300 }),
      )
      .mockResolvedValueOnce(jsonResponse(401, {}));
    vi.stubGlobal("fetch", fetchMock);

    await expect(useAuthStore.getState().authorizedFetch("/api/settings")).rejects.toThrow();
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });

  it("logout clears every session field", () => {
    useAuthStore.setState({
      accessToken: "a",
      refreshToken: "r",
      role: "owner",
      userId: "u",
      pendingPreAuthToken: "p",
    });

    useAuthStore.getState().logout();

    expect(useAuthStore.getState()).toMatchObject({
      accessToken: null,
      refreshToken: null,
      role: null,
      userId: null,
      pendingPreAuthToken: null,
    });
  });
});
