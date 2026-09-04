/** Client-side JWT payload reader — for display purposes only (which role
 * to render a nav item for, etc). This never verifies the signature; the
 * gateway is the only party that needs to (and does, on every request) —
 * reading an unverified claim client-side to decide what to *render* is
 * safe precisely because the server independently re-checks role/expiry
 * on every API call regardless of what the UI assumed. */
export interface JwtPayload {
  sub: string;
  role: "owner" | "trader" | "analyst" | "viewer";
  typ: string;
  exp: number;
  iat: number;
  jti: string;
}

export function decodeJwtPayload(token: string): JwtPayload {
  const [, payloadSegment] = token.split(".");
  if (!payloadSegment) throw new Error("not a JWT: missing payload segment");
  const base64 = payloadSegment.replace(/-/g, "+").replace(/_/g, "/");
  const padded = base64.padEnd(base64.length + ((4 - (base64.length % 4)) % 4), "=");
  const json = typeof atob === "function" ? atob(padded) : Buffer.from(padded, "base64").toString("utf8");
  return JSON.parse(json) as JwtPayload;
}

export function isExpired(payload: Pick<JwtPayload, "exp">, nowSeconds = Date.now() / 1000): boolean {
  return payload.exp <= nowSeconds;
}
