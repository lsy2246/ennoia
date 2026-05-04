import type { ApiErrorBody } from "@ennoia/contract";
import { createLogger } from "@ennoia/logs";

const logger = createLogger("api-client");
let runtimeDefaultRequestTimeoutMs = readGlobalRequestTimeoutMs();

export type FetchJsonInit = RequestInit & {
  timeoutMs?: number;
};

export function getApiBaseUrl() {
  const runtimeBaseUrl = (globalThis as { __ENNOIA_API_BASE_URL__?: string }).__ENNOIA_API_BASE_URL__;
  if (runtimeBaseUrl) {
    return runtimeBaseUrl;
  }
  if (import.meta.env.DEV && globalThis.location?.origin) {
    return globalThis.location.origin;
  }
  return import.meta.env.VITE_ENNOIA_API_URL ?? globalThis.location?.origin ?? "";
}

export function setApiClientRequestTimeout(timeoutMs: number | null | undefined) {
  runtimeDefaultRequestTimeoutMs = normalizeTimeoutMs(timeoutMs) ?? readGlobalRequestTimeoutMs();
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
    public traceId?: string | null,
    public details?: ApiErrorBody["details"],
    public retryable?: boolean,
  ) {
    super(message);
  }

  override toString() {
    return this.message;
  }
}

export async function fetchJson<T>(path: string, init?: FetchJsonInit): Promise<T> {
  const headers = new Headers(init?.headers);
  const method = (init?.method ?? "GET").toUpperCase();
  if (shouldAttachJsonContentType(method, init?.body, headers)) {
    headers.set("content-type", "application/json");
  }

  const timeoutMs = init?.timeoutMs ?? runtimeDefaultRequestTimeoutMs;
  const { signal, cleanup } = withRequestTimeout(init?.signal, timeoutMs);

  let response: Response;
  try {
    response = await fetch(apiUrl(path), {
      ...init,
      headers,
      signal,
    });
  } catch (error) {
    cleanup();
    if (isAbortError(error) && timeoutMs != null) {
      throw new ApiError(
        408,
        "TIMEOUT",
        `request timeout: ${method} ${path} after ${timeoutMs}ms`,
      );
    }
    throw error;
  }
  cleanup();

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
        trace_id: parsed.trace_id,
      });
      throw new ApiError(
        response.status,
        parsed.code,
        parsed.message || `request failed: ${response.status}`,
        parsed.request_id,
        parsed.trace_id,
        parsed.details,
        parsed.retryable,
      );
    }
    throw new ApiError(response.status, "INTERNAL", body || `request failed: ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

function readGlobalRequestTimeoutMs() {
  const globalTimeout = (globalThis as { __ENNOIA_API_REQUEST_TIMEOUT_MS__?: unknown })
    .__ENNOIA_API_REQUEST_TIMEOUT_MS__;
  return globalTimeout == null ? null : normalizeTimeoutMs(globalTimeout);
}

function normalizeTimeoutMs(value: unknown) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return null;
  }
  return Math.max(1000, Math.trunc(parsed));
}

function withRequestTimeout(sourceSignal: AbortSignal | null | undefined, timeoutMs: number | null) {
  const controller = new AbortController();
  const timeout = timeoutMs == null
    ? null
    : setTimeout(() => controller.abort(new DOMException("Request timed out", "TimeoutError")), timeoutMs);
  const abortFromSource = () => controller.abort(sourceSignal?.reason);

  if (sourceSignal?.aborted) {
    abortFromSource();
  } else if (sourceSignal) {
    sourceSignal.addEventListener("abort", abortFromSource, { once: true });
  }

  return {
    signal: controller.signal,
    cleanup: () => {
      if (timeout != null) {
        clearTimeout(timeout);
      }
      if (sourceSignal) {
        sourceSignal.removeEventListener("abort", abortFromSource);
      }
    },
  };
}

function isAbortError(error: unknown) {
  return error instanceof DOMException
    ? error.name === "AbortError" || error.name === "TimeoutError"
    : error instanceof Error && error.name === "AbortError";
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

export function toQueryString(input: Record<string, string | number | boolean | null | undefined>) {
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

