import { create } from "zustand";

import { apiJson, ApiError, type RequestOptions } from "../lib/api-client";
import { decodeJwtPayload, type JwtPayload } from "../lib/jwt";

export type Role = JwtPayload["role"];

interface TokenPair {
  accessToken: string;
  refreshToken: string;
  expiresInSeconds: number;
}

interface AuthState {
  accessToken: string | null;
  refreshToken: string | null;
  role: Role | null;
  userId: string | null;
  /** Set between a successful password check and a successful TOTP check —
   * §11.1's two-step login flow needs somewhere to hold this in between. */
  pendingPreAuthToken: string | null;

  submitPassword: (username: string, password: string) => Promise<void>;
  verifyTotp: (totpCode: string) => Promise<void>;
  /** Re-verifies TOTP to mint a short-lived step-up token for one
   * sensitive action (§13) — never stored in state, since it's meant to be
   * used once, immediately, not cached. */
  requestStepUp: (totpCode: string) => Promise<string>;
  refreshAccessToken: () => Promise<void>;
  logout: () => void;
  /** `apiJson` with this store's access token attached, retrying exactly
   * once through a token refresh on a 401. */
  authorizedFetch: <T>(path: string, options?: RequestOptions) => Promise<T>;
}

export const useAuthStore = create<AuthState>((set, get) => ({
  accessToken: null,
  refreshToken: null,
  role: null,
  userId: null,
  pendingPreAuthToken: null,

  async submitPassword(username, password) {
    const { preAuthToken } = await apiJson<{ preAuthToken: string }>("/api/auth/login", {
      method: "POST",
      body: { username, password },
    });
    set({ pendingPreAuthToken: preAuthToken });
  },

  async verifyTotp(totpCode) {
    const preAuthToken = get().pendingPreAuthToken;
    if (!preAuthToken) throw new Error("call submitPassword before verifyTotp");
    const tokens = await apiJson<TokenPair>("/api/auth/totp", {
      method: "POST",
      body: { preAuthToken, totpCode },
    });
    const payload = decodeJwtPayload(tokens.accessToken);
    set({
      accessToken: tokens.accessToken,
      refreshToken: tokens.refreshToken,
      role: payload.role,
      userId: payload.sub,
      pendingPreAuthToken: null,
    });
  },

  async requestStepUp(totpCode) {
    const accessToken = get().accessToken;
    if (!accessToken) throw new Error("not logged in");
    const { stepUpToken } = await apiJson<{ stepUpToken: string }>("/api/auth/step-up", {
      method: "POST",
      accessToken,
      body: { totpCode },
    });
    return stepUpToken;
  },

  async refreshAccessToken() {
    const refreshToken = get().refreshToken;
    if (!refreshToken) throw new Error("no refresh token to refresh with");
    const tokens = await apiJson<TokenPair>("/api/auth/refresh", {
      method: "POST",
      body: { refreshToken },
    });
    const payload = decodeJwtPayload(tokens.accessToken);
    set({
      accessToken: tokens.accessToken,
      refreshToken: tokens.refreshToken,
      role: payload.role,
      userId: payload.sub,
    });
  },

  logout() {
    set({ accessToken: null, refreshToken: null, role: null, userId: null, pendingPreAuthToken: null });
  },

  async authorizedFetch<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const accessToken = get().accessToken;
    try {
      return await apiJson<T>(path, { ...options, accessToken: accessToken ?? undefined });
    } catch (error) {
      if (error instanceof ApiError && error.status === 401 && get().refreshToken) {
        await get().refreshAccessToken();
        return apiJson<T>(path, { ...options, accessToken: get().accessToken ?? undefined });
      }
      throw error;
    }
  },
}));
