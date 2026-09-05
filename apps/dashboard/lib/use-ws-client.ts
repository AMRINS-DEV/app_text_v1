"use client";

import { useEffect, useState } from "react";

import { GATEWAY_WS_URL } from "./api-client";
import { createWsClient, type WsClient } from "./ws-client";
import { useAuthStore } from "../stores/auth";

/** One `/ws/stream` connection per authenticated session, torn down and
 * recreated whenever the access token changes (login/refresh/logout).
 * Returns `null` until the connection exists — consumers subscribe inside
 * their own `useEffect` keyed on this return value, which only becomes
 * non-null (triggering that effect) once a real client is ready. */
export function useWsClient(): WsClient | null {
  const accessToken = useAuthStore((state) => state.accessToken);
  const [client, setClient] = useState<WsClient | null>(null);

  useEffect(() => {
    if (!accessToken) {
      setClient(null);
      return;
    }
    const created = createWsClient(`${GATEWAY_WS_URL}/ws/stream`, accessToken);
    setClient(created);
    return () => {
      created.close();
      setClient(null);
    };
  }, [accessToken]);

  return client;
}
