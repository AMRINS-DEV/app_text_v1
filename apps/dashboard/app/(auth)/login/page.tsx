/**
 * Login page. Real auth flow (JWT+refresh, TOTP) lands in Phase 4 once
 * `services/gateway`'s AuthModule implements the endpoint (§11.1, §13).
 */
export default function LoginPage() {
  return (
    <main className="flex min-h-screen items-center justify-center">
      <div className="rounded-lg border p-8">
        <h1 className="text-xl font-semibold">TradeOS</h1>
        <p className="mt-2 text-sm text-neutral-500">Login — Phase 4 scope.</p>
      </div>
    </main>
  );
}
