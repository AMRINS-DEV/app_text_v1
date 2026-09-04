import { NextResponse } from "next/server";

/**
 * Thin route handler -> gateway (§4). Proxies to the NestJS gateway once
 * deployed; for now just proves the app/api convention builds.
 */
export function GET() {
  return NextResponse.json({ status: "ok", service: "dashboard" });
}
