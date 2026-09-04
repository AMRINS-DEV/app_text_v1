import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  transpilePackages: ["@tradeos/chart-engine", "@tradeos/schemas", "@tradeos/ui"],
  experimental: {
    // Client compute stays in Web Workers + WASM per §3.1/§12.2 — no special
    // Next config needed yet; this is a placeholder for the WASM loader
    // rewrite once packages/chart-engine's WASM build lands (Phase 4).
  },
};

export default nextConfig;
