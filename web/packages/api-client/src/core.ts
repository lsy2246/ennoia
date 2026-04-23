import type { ApiErrorBody } from "@ennoia/contract";
import { createLogger } from "@ennoia/observability";

const logger = createLogger("api-client");

export function getApiBaseUrl() {
  const runtimeBaseUrl = (globalThis as { __ENNOIA_API_BASE_URL__?: string }).__ENNOIA_API_BASE_URL__;
  return runtimeBaseUrl ?? import.meta.env.VITE_ENNOIA_API_URL ?? globalThis.location?.origin ?? "";
}

export function apiUrl(path: string) {
  const baseUrl = getApiBaseUrl();
  return baseUrl ? `${baseUrl}${path}` : path;
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: ApiErrorBody["code"],
    message: string,
    public requestId?: string | null,
  ) {
    super(message);
  }
}

export async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  const method = (init?.method ?? "GET").toUpperCase();
  if (shouldAttachJsonContentType(method, init?.body, headers)) {
    headers.set("content-type", "application/json");
  }

  const response = await fetch(apiUrl(path), {
    ...init,
    headers,
  });

  if (!response.ok) {
    const body = await response.text().catch(() => "");
    let parsed: ApiErrorBody | null;
    try {
      parsed = JSON.parse(body) as ApiErrorBody;
    } catch {
      parsed = null;
    }
    if (parsed) {
      logger.warn("request failed", {
        path,
        status: response.status,
        code: parsed.code,
        request_id: parsed.request_id,
      });
      throw new ApiError(
        response.status,
        parsed.code,
        parsed.message || `request failed: ${response.status}`,
        parsed.request_id,
      );
    }
    throw new ApiError(response.status, "INTERNAL", body || `request failed: ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

function shouldAttachJsonContentType(
  method: string,
  body: RequestInit["body"],
  headers: Headers,
) {
  if (headers.has("content-type")) {
    return false;
  }
  if (method === "GET" || method === "HEAD" || body == null) {
    return false;
  }
  if (typeof FormData !== "undefined" && body instanceof FormData) {
    return false;
  }
  if (typeof URLSearchParams !== "undefined" && body instanceof URLSearchParams) {
    return false;
  }
  if (typeof Blob !== "undefined" && body instanceof Blob) {
    return false;
  }
  if (body instanceof ArrayBuffer || ArrayBuffer.isView(body)) {
    return false;
  }
  return true;
}

export function toQueryString(input: Record<string, string | number | null | undefined>) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(input)) {
    if (value === undefined || value === null || value === "") {
      continue;
    }
    params.set(key, String(value));
  }
  const qs = params.toString();
  return qs ? `?${qs}` : "";
}

