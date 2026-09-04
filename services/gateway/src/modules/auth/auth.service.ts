import { Injectable, UnauthorizedException } from "@nestjs/common";
import * as bcrypt from "bcryptjs";

import { UsersStore } from "./users.store";
import { TokenService } from "./token.service";
import { verifyTotp } from "./totp";

export interface LoginResult {
  preAuthToken: string;
  expiresInSeconds: number;
}

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  expiresInSeconds: number;
}

/** §11.1's login → TOTP → tokens flow, plus refresh rotation and step-up. */
@Injectable()
export class AuthService {
  constructor(
    private readonly users: UsersStore,
    private readonly tokens: TokenService,
  ) {}

  login(username: string, password: string): LoginResult {
    const user = this.users.findByUsername(username);
    if (!user || !bcrypt.compareSync(password, user.passwordHash)) {
      throw new UnauthorizedException("invalid username or password");
    }
    const { token, expiresInSeconds } = this.tokens.sign(user.id, user.role, "preauth");
    return { preAuthToken: token, expiresInSeconds };
  }

  verifyTotpAndIssueTokens(preAuthToken: string, totpCode: string): TokenPair {
    const claims = this.tokens.verify(preAuthToken, "preauth");
    const user = this.users.findById(claims.sub);
    if (!user) throw new UnauthorizedException("unknown user");
    if (!verifyTotp(user.totpSecret, totpCode)) {
      throw new UnauthorizedException("invalid TOTP code");
    }
    return this.issueTokenPair(user.id, user.role);
  }

  refresh(refreshToken: string): TokenPair {
    const claims = this.tokens.verify(refreshToken, "refresh");
    const user = this.users.findById(claims.sub);
    if (!user) throw new UnauthorizedException("unknown user");
    this.tokens.revokeRefresh(claims.jti);
    return this.issueTokenPair(user.id, user.role);
  }

  /** Re-verifies TOTP against an already-authenticated user to mint a
   * short-lived step-up token (§13: "step-up auth (2FA re-prompt) for mode
   * change, kill-switch disable, risk-limit increase, and manual order
   * placement"). */
  stepUp(userId: string, totpCode: string): { stepUpToken: string; expiresInSeconds: number } {
    const user = this.users.findById(userId);
    if (!user) throw new UnauthorizedException("unknown user");
    if (!verifyTotp(user.totpSecret, totpCode)) {
      throw new UnauthorizedException("invalid TOTP code");
    }
    const { token, expiresInSeconds } = this.tokens.sign(user.id, user.role, "stepup");
    return { stepUpToken: token, expiresInSeconds };
  }

  private issueTokenPair(userId: string, role: Parameters<TokenService["sign"]>[1]): TokenPair {
    const access = this.tokens.sign(userId, role, "access");
    const refresh = this.tokens.sign(userId, role, "refresh");
    return { accessToken: access.token, refreshToken: refresh.token, expiresInSeconds: access.expiresInSeconds };
  }
}
