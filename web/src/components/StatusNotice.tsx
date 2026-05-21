import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import {
  resolveErrorAutoDismissMs,
  resolvePauseNotificationsOnHover,
  resolveSuccessAutoDismissMs,
} from "@/lib/uiDefaults";
import { useUiHelpers } from "@/stores/ui";

type StatusNoticeTone = "error" | "success";

const STATUS_NOTICE_ROOT_ID = "ennoia-status-notice-root";

function ensureStatusNoticeRoot() {
  if (typeof document === "undefined") {
    return null;
  }

  const existing = document.getElementById(STATUS_NOTICE_ROOT_ID);
  if (existing) {
    return existing;
  }

  const root = document.createElement("div");
  root.id = STATUS_NOTICE_ROOT_ID;
  root.className = "status-toast-root";
  document.body.appendChild(root);
  return root;
}

export function StatusNotice({
  message,
  tone,
  onDismiss,
  durationMs,
}: {
  message?: string | null;
  tone: StatusNoticeTone;
  onDismiss?: () => void;
  durationMs?: number;
}) {
  const { t } = useUiHelpers();
  const [target, setTarget] = useState<HTMLElement | null>(null);
  const [isHovering, setIsHovering] = useState(false);
  const [timerCycle, setTimerCycle] = useState(0);
  const pauseOnHover = resolvePauseNotificationsOnHover(undefined);
  const autoDismissDelay = useMemo(
    () =>
      durationMs
      ?? (tone === "error"
        ? resolveErrorAutoDismissMs(undefined)
        : resolveSuccessAutoDismissMs(undefined))
      ?? null,
    [durationMs, tone],
  );

  useEffect(() => {
    setTarget(ensureStatusNoticeRoot());
  }, []);

  useEffect(() => {
    setIsHovering(false);
    setTimerCycle((current) => current + 1);
  }, [message]);

  useEffect(() => {
    if (!message || !onDismiss || autoDismissDelay == null || (pauseOnHover && isHovering)) {
      return;
    }
    const timer = window.setTimeout(() => {
      onDismiss();
    }, autoDismissDelay);
    return () => window.clearTimeout(timer);
  }, [autoDismissDelay, isHovering, message, onDismiss, pauseOnHover]);

  if (!message || !target) {
    return null;
  }

  return createPortal(
    <section
      className={`status-toast status-toast--${tone}`}
      role={tone === "error" ? "alert" : "status"}
      aria-live={tone === "error" ? "assertive" : "polite"}
      aria-atomic="true"
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => {
        setIsHovering(false);
        setTimerCycle((current) => current + 1);
      }}
      style={
        autoDismissDelay == null
          ? undefined
          : ({ "--status-toast-duration": `${autoDismissDelay}ms` } as CSSProperties)
      }
    >
      {autoDismissDelay != null ? (
        <div
          key={`${tone}:${message}:${timerCycle}`}
          className={`status-toast__progress ${pauseOnHover && isHovering ? "status-toast__progress--paused" : ""}`}
          aria-hidden="true"
        />
      ) : null}
      <div className="status-toast__copy">{message}</div>
      {onDismiss ? (
        <button
          type="button"
          className="status-toast__close"
          onClick={onDismiss}
          aria-label={t("web.common.close_notice", "关闭提示")}
        >
          {t("web.common.close", "关闭")}
        </button>
      ) : null}
    </section>,
    target,
  );
}
