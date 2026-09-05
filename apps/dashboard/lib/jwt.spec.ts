import { describe, expect, it } from "vitest";

import { decodeJwtPayload, isExpired } from "./jwt";

function base64url(input: string): string {
  return Buffer.from(input, "utf8").toString("base64").replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fakeJwt(payload: Record<string, unknown>): string {
  const header = base64url(JSON.stringify({ alg: "HS256", typ: "JWT" }));
  const body = base64url(JSON.stringify(payload));
  return `${header}.${body}.fake-signature`;
}

describe("decodeJwtPayload", () => {
  it("decodes a well-formed JWT's payload", () => {
    const token = fakeJwt({ sub: "user-1", role: "trader", typ: "access", exp: 123, iat: 100, jti: "abc" });
    expect(decodeJwtPayload(token)).toEqual({
      sub: "user-1",
      role: "trader",
      typ: "access",
      exp: 123,
      iat: 100,
      jti: "abc",
    });
  });

  it("handles a payload segment needing base64 padding", () => {
    // A payload whose base64 length isn't a multiple of 4 forces the padding path.
    const token = fakeJwt({ sub: "x", role: "viewer", typ: "access", exp: 1, iat: 1, jti: "j" });
    expect(() => decodeJwtPayload(token)).not.toThrow();
  });

  it("throws for a string with no payload segment", () => {
    expect(() => decodeJwtPayload("not-a-jwt")).toThrow();
  });
});

describe("isExpired", () => {
  it("is false when exp is in the future", () => {
    expect(isExpired({ exp: 1000 }, 500)).toBe(false);
  });

  it("is true when exp is now or in the past", () => {
    expect(isExpired({ exp: 500 }, 500)).toBe(true);
    expect(isExpired({ exp: 500 }, 501)).toBe(true);
  });
});
