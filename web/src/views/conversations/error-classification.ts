export type ConversationFailureSource =
  | "file_access"
  | "approval"
  | "permission"
  | "provider"
  | "timeout"
  | "configuration"
  | "extension"
  | "system"
  | "agent";

export type ConversationFailureClassification = {
  source: ConversationFailureSource;
  code?: string;
  summary: string;
  detail: string;
};

const FILE_ACCESS_PATTERNS = [
  "file access only accepts configured virtual roots",
  "path cannot escape the selected file access root",
  "path must stay inside the selected file access root",
];

const PROVIDER_PATTERNS = [
  "openai api key is missing",
  "openai request failed",
  "upstream returned",
  "provider returned empty",
  "当前上游不支持",
  "模型发现接口",
];

const TIMEOUT_PATTERNS = [
  "request timeout:",
  "request timeout after",
  "timed out",
  "deadline exceeded",
];

const CONFIGURATION_PATTERNS = [
  "provider invoke requires params.model_endpoint",
  "missing field `display_name`",
  "missing field",
  "invalid configuration",
];

const EXTENSION_PATTERNS = [
  "extension rpc failed",
  "method_not_found",
  "conversation worker method",
  "parse extension record",
];

const SYSTEM_PATTERNS = [
  "internal server error",
];

const APPROVAL_PATTERNS = [
  "approval required:",
  "等待审批",
];

const PERMISSION_PATTERNS = [
  "permission denied",
  "权限已拒绝",
];

const GENERIC_ERROR_PATTERNS = [
  "error:",
  "exception:",
  "panic:",
  "request failed:",
];

function includesAny(normalized: string, patterns: string[]) {
  return patterns.some((pattern) => normalized.includes(pattern));
}

function summarizeLines(lines: string[]) {
  return lines[0] ?? "";
}

export function classifyConversationFailure(message: string): ConversationFailureClassification | null {
  const detail = message.trim();
  if (!detail) {
    return null;
  }

  const normalized = detail.toLowerCase();
  const lines = detail
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);

  const firstLine = lines[0] ?? "";
  const restLines = lines.slice(1);

  if (firstLine === "文件访问路径已拦截") {
    return {
      source: "file_access",
      code: "file_access_path_restricted",
      summary: summarizeLines(restLines) || firstLine,
      detail,
    };
  }

  if (firstLine === "上游模型错误") {
    return {
      source: "provider",
      summary: summarizeLines(restLines) || firstLine,
      detail,
    };
  }

  if (firstLine === "请求超时") {
    return {
      source: "timeout",
      summary: summarizeLines(restLines) || firstLine,
      detail,
    };
  }

  if (firstLine === "配置错误") {
    return {
      source: "configuration",
      summary: summarizeLines(restLines) || firstLine,
      detail,
    };
  }

  if (firstLine === "扩展运行错误") {
    return {
      source: "extension",
      summary: summarizeLines(restLines) || firstLine,
      detail,
    };
  }

  if (firstLine === "系统错误" || firstLine === "系统内部错误") {
    return {
      source: "system",
      summary: summarizeLines(restLines) || firstLine,
      detail,
    };
  }

  if (includesAny(normalized, FILE_ACCESS_PATTERNS)) {
    return {
      source: "file_access",
      code: "file_access_path_restricted",
      summary: summarizeLines(lines),
      detail,
    };
  }

  if (includesAny(normalized, APPROVAL_PATTERNS)) {
    return {
      source: "approval",
      summary: summarizeLines(lines),
      detail,
    };
  }

  if (includesAny(normalized, PERMISSION_PATTERNS)) {
    return {
      source: "permission",
      summary: summarizeLines(lines),
      detail,
    };
  }

  if (includesAny(normalized, PROVIDER_PATTERNS)) {
    return {
      source: "provider",
      summary: summarizeLines(lines),
      detail,
    };
  }

  if (includesAny(normalized, TIMEOUT_PATTERNS)) {
    return {
      source: "timeout",
      summary: summarizeLines(lines),
      detail,
    };
  }

  if (includesAny(normalized, CONFIGURATION_PATTERNS)) {
    return {
      source: "configuration",
      summary: summarizeLines(lines),
      detail,
    };
  }

  if (includesAny(normalized, EXTENSION_PATTERNS)) {
    return {
      source: "extension",
      summary: summarizeLines(lines),
      detail,
    };
  }

  if (includesAny(normalized, SYSTEM_PATTERNS)) {
    return {
      source: "system",
      summary: summarizeLines(lines),
      detail,
    };
  }

  if (
    GENERIC_ERROR_PATTERNS.some((pattern) => normalized.startsWith(pattern))
    || normalized.endsWith(" failed")
  ) {
    return {
      source: "system",
      summary: summarizeLines(lines),
      detail,
    };
  }

  return null;
}

export function isLikelyFailureMessage(message: string) {
  return classifyConversationFailure(message) !== null;
}
