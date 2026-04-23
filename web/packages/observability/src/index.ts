export type LogLevel = "debug" | "info" | "warn" | "error";

export type LoggerFields = Record<string, unknown>;

export interface Logger {
  debug(message: string, fields?: LoggerFields): void;
  info(message: string, fields?: LoggerFields): void;
  warn(message: string, fields?: LoggerFields): void;
  error(message: string, fields?: LoggerFields): void;
}

function resolveMinLevel(): LogLevel {
  const value =
    import.meta.env.VITE_ENNOIA_LOG_LEVEL ??
    import.meta.env.VITE_LOG_LEVEL ??
    import.meta.env.ENNOIA_LOG_LEVEL ??
    "info";
  switch (String(value).trim().toLowerCase()) {
    case "debug":
      return "debug";
    case "warn":
      return "warn";
    case "error":
      return "error";
    default:
      return "info";
  }
}

const LOG_LEVEL_ORDER: Record<LogLevel, number> = {
  debug: 10,
  info: 20,
  warn: 30,
  error: 40,
};

function emit(level: LogLevel, scope: string, message: string, fields?: LoggerFields) {
  if (LOG_LEVEL_ORDER[level] < LOG_LEVEL_ORDER[resolveMinLevel()]) {
    return;
  }
  const payload = {
    timestamp: new Date().toISOString(),
    level,
    scope,
    message,
    ...(fields ?? {}),
  };

  const line = `[${payload.level}] ${payload.scope}: ${payload.message}`;
  switch (level) {
    case "debug":
      console.debug(line, payload);
      break;
    case "info":
      console.info(line, payload);
      break;
    case "warn":
      console.warn(line, payload);
      break;
    case "error":
      console.error(line, payload);
      break;
  }
}

export function createLogger(scope: string): Logger {
  return {
    debug(message, fields) {
      emit("debug", scope, message, fields);
    },
    info(message, fields) {
      emit("info", scope, message, fields);
    },
    warn(message, fields) {
      emit("warn", scope, message, fields);
    },
    error(message, fields) {
      emit("error", scope, message, fields);
    },
  };
}
