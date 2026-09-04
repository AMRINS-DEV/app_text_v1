"use client";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { StepUpPrompt } from "../../../components/step-up-prompt";
import { useWsClient } from "../../../lib/use-ws-client";
import { useAuthStore } from "../../../stores/auth";

interface Position {
  id: string;
  symbol: string;
  side: "buy" | "sell";
  quantity: number;
  entryPrice: number;
  sl: number;
  tp: number;
  openedAt: number;
}

interface AccountSnapshot {
  mode: "live" | "paper" | "halted";
  killSwitchEngaged: boolean;
  positions: Position[];
}

type PendingAction = "kill-switch-reset" | "mode-live" | "mode-paper" | null;

/** Positions, mode switcher and kill switch (§4, §9.1, §9.5, §13). The
 * kill switch itself fires immediately (no step-up, by design — see
 * `TradingService.killSwitch`'s doc comment); resetting it and changing
 * mode both go through `StepUpPrompt`. */
export default function TradingPage() {
  const authorizedFetch = useAuthStore((state) => state.authorizedFetch);
  const requestStepUp = useAuthStore((state) => state.requestStepUp);
  const ws = useWsClient();
  const queryClient = useQueryClient();

  const { data: initialPositions } = useQuery({
    queryKey: ["positions"],
    queryFn: () => authorizedFetch<Position[]>("/api/trading/positions"),
  });
  const [account, setAccount] = useState<AccountSnapshot | null>(null);
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [killSwitchResult, setKillSwitchResult] = useState<string | null>(null);

  useEffect(() => {
    if (!ws) return;
    return ws.subscribe<AccountSnapshot>("positions", (frame) => setAccount(frame.payload));
  }, [ws]);

  const positions = account?.positions ?? initialPositions ?? [];

  async function handleKillSwitch() {
    const result = await authorizedFetch<{ flattenedCount: number; elapsedMs: number }>("/api/trading/kill-switch", {
      method: "POST",
    });
    setKillSwitchResult(`Flattened ${result.flattenedCount} position(s) in ${result.elapsedMs.toFixed(2)}ms`);
    await queryClient.invalidateQueries({ queryKey: ["positions"] });
  }

  async function confirmStepUp(totpCode: string) {
    const stepUpToken = await requestStepUp(totpCode);
    if (pendingAction === "kill-switch-reset") {
      await authorizedFetch("/api/trading/kill-switch/reset", { method: "POST", stepUpToken });
    } else if (pendingAction === "mode-live" || pendingAction === "mode-paper") {
      const mode = pendingAction === "mode-live" ? "live" : "paper";
      await authorizedFetch("/api/trading/mode", { method: "POST", stepUpToken, body: { mode } });
    }
    setPendingAction(null);
    await queryClient.invalidateQueries({ queryKey: ["positions"] });
  }

  return (
    <main className="p-8">
      <h1 className="text-2xl font-semibold">Trading</h1>

      <div className="mt-4 flex items-center gap-3 text-sm">
        <span className="text-neutral-400">Mode: {account?.mode ?? "—"}</span>
        {account?.killSwitchEngaged && (
          <span className="rounded bg-red-900/50 px-2 py-0.5 text-xs text-red-300">Kill switch engaged</span>
        )}
      </div>

      <div className="mt-4 flex gap-2">
        <button onClick={handleKillSwitch} className="rounded bg-red-600 px-3 py-2 text-sm font-medium">
          Kill switch
        </button>
        <button
          onClick={() => setPendingAction("kill-switch-reset")}
          className="rounded border border-neutral-700 px-3 py-2 text-sm"
        >
          Reset kill switch
        </button>
        <button
          onClick={() => setPendingAction("mode-live")}
          className="rounded border border-neutral-700 px-3 py-2 text-sm"
        >
          Set live
        </button>
        <button
          onClick={() => setPendingAction("mode-paper")}
          className="rounded border border-neutral-700 px-3 py-2 text-sm"
        >
          Set paper
        </button>
      </div>

      {killSwitchResult && <p className="mt-2 text-xs text-neutral-500">{killSwitchResult}</p>}

      <table className="mt-6 w-full text-sm">
        <thead>
          <tr className="text-left text-neutral-500">
            <th className="pb-2">Symbol</th>
            <th className="pb-2">Side</th>
            <th className="pb-2">Qty</th>
            <th className="pb-2">Entry</th>
            <th className="pb-2">SL</th>
            <th className="pb-2">TP</th>
          </tr>
        </thead>
        <tbody>
          {positions.map((position) => (
            <tr key={position.id} className="border-t border-neutral-800">
              <td className="py-2">{position.symbol}</td>
              <td className="py-2">{position.side}</td>
              <td className="py-2">{position.quantity}</td>
              <td className="py-2">{position.entryPrice}</td>
              <td className="py-2">{position.sl}</td>
              <td className="py-2">{position.tp}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {positions.length === 0 && <p className="mt-4 text-sm text-neutral-500">No open positions.</p>}

      {pendingAction && <StepUpPrompt onConfirm={confirmStepUp} onCancel={() => setPendingAction(null)} />}
    </main>
  );
}
