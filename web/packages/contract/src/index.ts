export type ErrorCode =
  | "BAD_REQUEST"
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "CONFLICT"
  | "RATE_LIMITED"
  | "PAYLOAD_TOO_LARGE"
  | "TIMEOUT"
  | "INTERNAL";

export type ApiErrorBody = {
  code: ErrorCode;
  message: string;
  request_id?: string | null;
  trace_id?: string | null;
  details?: Record<string, unknown> | null;
  retryable: boolean;
};
