import { randomUUID } from "node:crypto";

import { Inject, Injectable, UnauthorizedException } from "@nestjs/common";
import * as jwt from "jsonwebtoken";

import { Role } from "../../common/roles";

export const JWT_SECRET = "JWT_SECRET";

export type TokenType = "preauth" | "access" | "refresh" | "stepup";

export interface TokenClaims {
  sub: string;
  role: Role;
  typ: TokenType;
  jti: string;
}

const EXPIRY_SECONDS: Record<TokenType, number> = {
  preauth: 2 * 60,
  access: 5 * 60,
  refresh: 7 * 24 * 60 * 60,
  stepup: 2 * 60,
};

/**
 * §11.1: "JWT (short-lived) + refresh... TOTP 2FA... step-up auth for
 * trading actions." Four token types share one signer, distinguished by the
 * `typ` claim: `preauth` (issued after password check, consumed by the TOTP
 * step), `access`, `refresh`, and `stepup` (issued after a *second* TOTP
 * check, required by the step-up guard on sensitive trading actions).
 * Refresh tokens are rotated on use; the revoked-`jti` set is in-memory —
 * this sandbox has no Redis to back it, so revocations don't survive a
 * gateway restart (documented, not silently assumed away).
 */
@Injectable()
export class TokenService {
  private readonly revokedRefreshJtis = new Set<string>();

  constructor(@Inject(JWT_SECRET) private readonly secret: string) {}

  sign(sub: string, role: Role, typ: TokenType): { token: string; jti: string; expiresInSeconds: number } {
    const jti = randomUUID();
    const expiresInSeconds = EXPIRY_SECONDS[typ];
    const token = jwt.sign({ sub, role, typ, jti } satisfies TokenClaims, this.secret, {
      expiresIn: expiresInSeconds,
    });
    return { token, jti, expiresInSeconds };
  }

  verify(token: string, expectedType: TokenType): TokenClaims {
    let claims: TokenClaims;
    try {
      claims = jwt.verify(token, this.secret) as TokenClaims;
    } catch {
      throw new UnauthorizedException("invalid or expired token");
    }
    if (claims.typ !== expectedType) {
      throw new UnauthorizedException(`expected a ${expectedType} token, got ${claims.typ}`);
    }
    if (claims.typ === "refresh" && this.revokedRefreshJtis.has(claims.jti)) {
      throw new UnauthorizedException("refresh token has been rotated");
    }
    return claims;
  }

  revokeRefresh(jti: string): void {
    this.revokedRefreshJtis.add(jti);
  }
}
