import { useEffect, useRef } from "react";
import { RouterProvider } from "@tanstack/react-router";

import { reportFrontendLog } from "@ennoia/api-client";
import { router } from "@/router";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";
import { classifyConversationFailure } from "@/views/conversations/error-classification";

function reportRuntimeError(title: string, error: unknown) {
  void reportFrontendLog({
    level: "error",
    source: "frontend",
    title,
    summary: error instanceof Error ? error.message : String(error),
    details: error instanceof Error ? error.stack : undefined,
    at: new Date().toISOString(),
  }).catch(() => undefined);
}

window.addEventListener("error", (event) => {
  reportRuntimeError("window.error", event.error ?? event.message);
});

window.addEventListener("unhandledrejection", (event) => {
  reportRuntimeError("unhandledrejection", event.reason);
});

export function AppShell() {
  const runtimeHydrate = useRuntimeStore((state) => state.hydrate);
  const runtimeStatus = useRuntimeStore((state) => state.status);
  const runtimeError = useRuntimeStore((state) => state.error);
  const uiHydrate = useUiStore((state) => state.hydrate);
  const connectExtensionEvents = useUiStore((state) => state.connectExtensionEvents);
  const uiStatus = useUiStore((state) => state.status);
  const uiError = useUiStore((state) => state.error);
  const { t } = useUiHelpers();
  const autoRetryTimerRef = useRef<number | null>(null);
  const autoRetryAttemptsRef = useRef(0);

  useEffect(() => {
    runtimeHydrate();
    uiHydrate();
  }, [runtimeHydrate, uiHydrate]);

  useEffect(() => connectExtensionEvents(), [connectExtensionEvents]);

  useEffect(() => {
    if (!import.meta.env.DEV) {
      return;
    }

    if (runtimeStatus === "ready" && uiStatus === "ready") {
      autoRetryAttemptsRef.current = 0;
      if (autoRetryTimerRef.current != null) {
        window.clearTimeout(autoRetryTimerRef.current);
        autoRetryTimerRef.current = null;
      }
      return;
    }

    if (runtimeStatus !== "error" && uiStatus !== "error") {
      return;
    }

    if (autoRetryTimerRef.current != null) {
      return;
    }

    const nextAttempt = autoRetryAttemptsRef.current + 1;
    autoRetryAttemptsRef.current = nextAttempt;
    const delayMs = Math.min(1500 * nextAttempt, 5000);
    autoRetryTimerRef.current = window.setTimeout(() => {
      autoRetryTimerRef.current = null;
      void runtimeHydrate();
      void uiHydrate();
    }, delayMs);

    return () => {
      if (autoRetryTimerRef.current != null) {
        window.clearTimeout(autoRetryTimerRef.current);
        autoRetryTimerRef.current = null;
      }
    };
  }, [runtimeHydrate, runtimeStatus, uiHydrate, uiStatus]);

  if (
    runtimeStatus === "idle" ||
    runtimeStatus === "checking" ||
    uiStatus === "idle" ||
    uiStatus === "checking"
  ) {
    return (
      <div className="page page--centered">
        <p>{t("web.loading.connecting", "Connecting to Ennoia…")}</p>
      </div>
    );
  }

  if (runtimeStatus === "error" || uiStatus === "error") {
    const errorMessage = runtimeError ?? uiError ?? t("web.common.unknown", "未知错误");
    const classifiedError = classifyConversationFailure(errorMessage);
    const errorTitle = (() => {
      switch (classifiedError?.source) {
        case "provider":
          return t("web.conversations.upstream_error_title", "上游模型错误");
        case "timeout":
          return t("web.conversations.timeout_error_title", "请求超时");
        case "configuration":
          return t("web.conversations.configuration_error_title", "配置错误");
        case "extension":
          return t("web.conversations.extension_error_title", "扩展运行错误");
        case "sandbox":
          return t("web.conversations.sandbox_path_error_title", "沙盒路径已拦截");
        case "approval":
          return t("web.conversations.permission_approval_title", "等待审批");
        case "permission":
          return t("web.conversations.permission_error_title", "权限已拒绝");
        case "system":
          return t("web.conversations.system_error_title", "系统错误");
        default:
          return t("web.loading.connect_failed", "连接 Ennoia 失败");
      }
    })();
    const errorSummary = classifiedError?.summary ?? errorMessage;
    const errorDetail =
      classifiedError?.detail && classifiedError.detail !== errorSummary
        ? classifiedError.detail
        : null;
    return (
      <div className="page page--centered" style={{ gap: 12, textAlign: "center" }}>
        <p>{errorTitle}</p>
        <small style={{ maxWidth: 720, opacity: 0.82 }}>{errorSummary}</small>
        {errorDetail ? (
          <small style={{ maxWidth: 720, opacity: 0.62 }}>{errorDetail}</small>
        ) : null}
        <div style={{ display: "flex", gap: 8, justifyContent: "center" }}>
          <button
            type="button"
            className="btn"
            onClick={() => {
              void runtimeHydrate();
              void uiHydrate();
            }}
          >
            {t("web.common.retry", "重试")}
          </button>
        </div>
      </div>
    );
  }

  return <RouterProvider router={router} />;
}
