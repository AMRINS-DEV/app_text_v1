"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

import { ApiError } from "../../../lib/api-client";
import { useAuthStore } from "../../../stores/auth";

/** Real two-step login (§11.1: password, then TOTP 2FA) against the
 * gateway's AuthModule. Dev fixture accounts: owner/trader/analyst/viewer,
 * each with password `<username>-dev-password` (see
 * `services/gateway/src/modules/auth/users.store.ts`) — there is no real
 * user database behind this in this sandbox. */
export default function LoginPage() {
  const router = useRouter();
  const submitPassword = useAuthStore((state) => state.submitPassword);
  const verifyTotp = useAuthStore((state) => state.verifyTotp);
  const pendingPreAuthToken = useAuthStore((state) => state.pendingPreAuthToken);

  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [totpCode, setTotpCode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handlePasswordSubmit(event: React.FormEvent) {
    event.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await submitPassword(username, password);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Login failed");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleTotpSubmit(event: React.FormEvent) {
    event.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await verifyTotp(totpCode);
      router.replace("/");
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Invalid code");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-neutral-950">
      <div className="w-80 rounded-lg border border-neutral-800 p-8">
        <h1 className="text-xl font-semibold">TradeOS</h1>

        {!pendingPreAuthToken ? (
          <form onSubmit={handlePasswordSubmit} className="mt-6 flex flex-col gap-3">
            <input
              className="rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm"
              placeholder="Username"
              value={username}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setUsername(e.target.value)}
              autoFocus
            />
            <input
              className="rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm"
              placeholder="Password"
              type="password"
              value={password}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPassword(e.target.value)}
            />
            {error && <p className="text-sm text-red-400">{error}</p>}
            <button
              type="submit"
              disabled={submitting}
              className="rounded bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
            >
              Continue
            </button>
          </form>
        ) : (
          <form onSubmit={handleTotpSubmit} className="mt-6 flex flex-col gap-3">
            <p className="text-sm text-neutral-400">Enter your 6-digit authenticator code.</p>
            <input
              className="rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm tracking-widest"
              placeholder="000000"
              inputMode="numeric"
              maxLength={6}
              value={totpCode}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setTotpCode(e.target.value)}
              autoFocus
            />
            {error && <p className="text-sm text-red-400">{error}</p>}
            <button
              type="submit"
              disabled={submitting}
              className="rounded bg-blue-600 px-3 py-2 text-sm font-medium disabled:opacity-50"
            >
              Verify
            </button>
          </form>
        )}
      </div>
    </main>
  );
}
