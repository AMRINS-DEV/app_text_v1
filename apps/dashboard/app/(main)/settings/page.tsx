"use client";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { useAuthStore } from "../../../stores/auth";

interface RiskProfile {
  riskPerTradePct: number;
  fractionalKellyCap: number;
  maxDailyDrawdownPct: number;
  maxTotalDrawdownPct: number;
}

interface Settings {
  riskProfile: RiskProfile;
  allowedPairs: string[];
  defaultMode: "live" | "paper";
  modelRouting: Record<string, string>;
}

/** Risk profile, allowed pairs, default mode (§11.1). Provider keys
 * (write-only, masked reads — §13) and agent config/model routing UI
 * remain Phase 5+ scope, once secrets and the agent layer exist. */
export default function SettingsPage() {
  const authorizedFetch = useAuthStore((state) => state.authorizedFetch);
  const queryClient = useQueryClient();
  const { data } = useQuery({
    queryKey: ["settings"],
    queryFn: () => authorizedFetch<Settings>("/api/settings"),
  });

  const [form, setForm] = useState<Settings | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (data) setForm(data);
  }, [data]);

  async function handleSave(event: React.FormEvent) {
    event.preventDefault();
    if (!form) return;
    await authorizedFetch("/api/settings", { method: "PUT", body: form });
    await queryClient.invalidateQueries({ queryKey: ["settings"] });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  }

  if (!form) {
    return (
      <main className="p-8">
        <h1 className="text-2xl font-semibold">Settings</h1>
        <p className="mt-2 text-sm text-neutral-500">Loading…</p>
      </main>
    );
  }

  return (
    <main className="p-8">
      <h1 className="text-2xl font-semibold">Settings</h1>
      <form onSubmit={handleSave} className="mt-6 flex max-w-md flex-col gap-4">
        <NumberField
          label="Risk per trade (%)"
          value={form.riskProfile.riskPerTradePct}
          onChange={(v) => setForm({ ...form, riskProfile: { ...form.riskProfile, riskPerTradePct: v } })}
        />
        <NumberField
          label="Fractional Kelly cap"
          value={form.riskProfile.fractionalKellyCap}
          step={0.05}
          onChange={(v) => setForm({ ...form, riskProfile: { ...form.riskProfile, fractionalKellyCap: v } })}
        />
        <NumberField
          label="Max daily drawdown (%)"
          value={form.riskProfile.maxDailyDrawdownPct}
          onChange={(v) => setForm({ ...form, riskProfile: { ...form.riskProfile, maxDailyDrawdownPct: v } })}
        />
        <NumberField
          label="Max total drawdown (%)"
          value={form.riskProfile.maxTotalDrawdownPct}
          onChange={(v) => setForm({ ...form, riskProfile: { ...form.riskProfile, maxTotalDrawdownPct: v } })}
        />

        <label className="text-sm">
          <span className="text-neutral-400">Allowed pairs (comma-separated)</span>
          <input
            className="mt-1 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm"
            value={form.allowedPairs.join(",")}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              setForm({
                ...form,
                allowedPairs: e.target.value
                  .split(",")
                  .map((s: string) => s.trim())
                  .filter(Boolean),
              })
            }
          />
        </label>

        <label className="text-sm">
          <span className="text-neutral-400">Default mode</span>
          <select
            className="mt-1 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm"
            value={form.defaultMode}
            onChange={(e: React.ChangeEvent<HTMLSelectElement>) =>
              setForm({ ...form, defaultMode: e.target.value as "live" | "paper" })
            }
          >
            <option value="paper">Paper</option>
            <option value="live">Live</option>
          </select>
        </label>

        <button type="submit" className="rounded bg-blue-600 px-3 py-2 text-sm font-medium">
          Save
        </button>
        {saved && <p className="text-xs text-green-400">Saved.</p>}
      </form>
    </main>
  );
}

function NumberField({
  label,
  value,
  step = 0.1,
  onChange,
}: {
  label: string;
  value: number;
  step?: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="text-sm">
      <span className="text-neutral-400">{label}</span>
      <input
        type="number"
        step={step}
        className="mt-1 w-full rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm"
        value={value}
        onChange={(e: React.ChangeEvent<HTMLInputElement>) => onChange(Number(e.target.value))}
      />
    </label>
  );
}
