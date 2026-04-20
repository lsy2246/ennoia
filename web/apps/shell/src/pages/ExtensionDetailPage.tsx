import { useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import {
  detachExtensionWorkspace,
  getExtension,
  getExtensionLogs,
  type ExtensionDetail,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function ExtensionDetailPage() {
  const { extensionId } = useParams({ from: "/shell/extensions/$extensionId" });
  const { t } = useUiHelpers();
  const [detail, setDetail] = useState<ExtensionDetail | null>(null);
  const [logs, setLogs] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [nextDetail, nextLogs] = await Promise.all([
          getExtension(extensionId),
          getExtensionLogs(extensionId),
        ]);
        if (!cancelled) {
          setDetail(nextDetail);
          setLogs(nextLogs);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [extensionId]);

  if (error) {
    return <div className="page error">{error}</div>;
  }

  if (!detail) {
    return <div className="page">{t("shell.action.loading", "加载中…")}</div>;
  }

  return (
    <div className="page">
      <div className="surface-grid">
        <section className="surface-panel">
          <h1>{detail.name}</h1>
          <dl className="data-pairs">
            <dt>ID</dt>
            <dd>{detail.id}</dd>
            <dt>{t("shell.common.status", "状态")}</dt>
            <dd>{detail.health}</dd>
            <dt>{t("shell.extensions.version", "版本")}</dt>
            <dd>{detail.version}</dd>
            <dt>{t("shell.extensions.install_dir", "目录")}</dt>
            <dd>{detail.install_dir}</dd>
          </dl>
          <div className="button-row">
            <button
              className="danger"
              onClick={() =>
                void detachExtensionWorkspace(extensionId).catch((err) => setError(String(err)))
              }
            >
              {t("shell.extensions.detach", "卸载挂载")}
            </button>
          </div>
        </section>

        <section className="surface-panel">
          <h2>{t("shell.extensions.diagnostics", "诊断")}</h2>
          <div className="stack-list">
            {detail.diagnostics.map((item, index) => (
              <div key={`${item.level}:${index}`} className="execution-row">
                <strong>{item.level}</strong>
                <span>{item.summary}</span>
              </div>
            ))}
            {detail.diagnostics.length === 0 ? (
              <p className="muted">{t("shell.common.none_plain", "无")}</p>
            ) : null}
          </div>
        </section>
      </div>

      <section className="surface-panel">
        <h2>{t("shell.logs.title", "日志")}</h2>
        <pre className="log-view">{logs || t("shell.common.none_plain", "无")}</pre>
      </section>
    </div>
  );
}
