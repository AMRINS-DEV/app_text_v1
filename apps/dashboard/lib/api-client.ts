export const GATEWAY_HTTP_URL = process.env.NEXT_PUBLIC_GATEWAY_URL ?? "http://localhost:4000";
export const GATEWAY_WS_URL = process.env.NEXT_PUBLIC_GATEWAY_WS_URL ?? "ws://localhost:4000";

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

export interface RequestOptions extends Omit<RequestInit, "body"> {
  accessToken?: string;
  stepUpToken?: string;
  /** JSON-serialized automatically unless already a string. */
  body?: unknown;
}

/** Thin fetch wrapper: gateway base URL, bearer/step-up headers, JSON body
 * encoding. Token *refresh* lives in `stores/auth.ts`, which owns the
 * tokens this wrapper is handed. */
export async function apiFetch(path: string, options: RequestOptions = {}): Promise<Response> {
  const { accessToken, stepUpToken, body, headers: rawHeaders, ...rest } = options;
  const headers = new Headers(rawHeaders);
  if (accessToken) headers.set("Authorization", `Bearer ${accessToken}`);
  if (stepUpToken) headers.set("x-step-up-token", stepUpToken);

  let finalBody: BodyInit | undefined;
  if (body !== undefined && typeof body !== "string") {
    headers.set("Content-Type", "application/json");
    finalBody = JSON.stringify(body);
  } else {
    finalBody = body;
  }

  return fetch(`${GATEWAY_HTTP_URL}${path}`, { ...rest, headers, body: finalBody });
}

export async function apiJson<T>(path: string, options?: RequestOptions): Promise<T> {
  const res = await apiFetch(path, options);
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new ApiError(res.status, text || res.statusText);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}
