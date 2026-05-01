import { useEffect, useState } from "react";
import { createPortal } from "react-dom";

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
}: {
  message?: string | null;
  tone: StatusNoticeTone;
  onDismiss?: () => void;
}) {
  const [target, setTarget] = useState<HTMLElement | null>(null);

  useEffect(() => {
    setTarget(ensureStatusNoticeRoot());
  }, []);

  if (!message || !target) {
    return null;
  }

  return createPortal(
    <section
      className={`status-toast status-toast--${tone}`}
      role={tone === "error" ? "alert" : "status"}
      aria-live={tone === "error" ? "assertive" : "polite"}
      aria-atomic="true"
    >
      <div className="status-toast__copy">{message}</div>
      {onDismiss ? (
        <button
          type="button"
          className="status-toast__close"
          onClick={onDismiss}
          aria-label="关闭提示"
        >
          关闭
        </button>
      ) : null}
    </section>,
    target,
  );
}
