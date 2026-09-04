"use client";

import { useState } from "react";

/** §13's step-up (2FA re-prompt) modal for mode change / kill-switch
 * reset / risk-limit increase / manual order placement. Collects a fresh
 * TOTP code and hands it to `onConfirm`, which is expected to call
 * `useAuthStore().requestStepUp(code)` and then the actual action. */
export function StepUpPrompt({
  onConfirm,
  onCancel,
}: {
  onConfirm: (totpCode: string) => Promise<void>;
  onCancel: () => void;
}) {
  const [code, setCode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await onConfirm(code);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Verification failed");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="fixed inset-0 flex items-center justify-center bg-black/60">
      <form onSubmit={handleSubmit} className="w-72 rounded-lg border border-neutral-700 bg-neutral-900 p-6">
        <h2 className="text-sm font-semibold">Step-up verification required</h2>
        <p className="mt-1 text-xs text-neutral-400">Enter your 6-digit authenticator code to continue.</p>
        <input
          className="mt-3 w-full rounded border border-neutral-700 bg-neutral-950 px-3 py-2 text-sm tracking-widest"
          maxLength={6}
          inputMode="numeric"
          value={code}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCode(e.target.value)}
          autoFocus
        />
        {error && <p className="mt-2 text-xs text-red-400">{error}</p>}
        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="rounded px-3 py-1.5 text-xs text-neutral-400 hover:bg-neutral-800"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={submitting}
            className="rounded bg-blue-600 px-3 py-1.5 text-xs font-medium disabled:opacity-50"
          >
            Confirm
          </button>
        </div>
      </form>
    </div>
  );
}
