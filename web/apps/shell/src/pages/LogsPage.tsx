import { useEffect, useState } from "react";

import { listLogs, type LogRecord } from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useUiHelpers } from "@/stores/ui";

export function LogsPage() {
  const { t, formatDateTime } = useUiHelpers();
  const [limit, setLimit] = useState(50);
  const [logs, setLogs] = useState<LogRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      setLogs(await listLogs(limit));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, [limit]);

  if (loading) {
    return <div className="page"><p>{t("shell.loading.logs", "Loading logs…")}</p></div>;
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.logs.title", "Logs")}
        description={t(
          "shell.page.logs.description",
          "Review recent runtime audit records to understand stage transitions, decisions and gates.",
        )}
        meta={[`${logs.length} ${t("shell.meta.total", "total")}`]}
        actions={
          <div className="inline-actions">
            <select value={limit} onChange={(event) => setLimit(Number(event.target.value))}>
              <option value={25}>25</option>
              <option value={50}>50</option>
              <option value={100}>100</option>
            </select>
            <button className="secondary" onClick={() => void refresh()}>
              {t("shell.action.refresh", "Refresh")}
            </button>
          </div>
        }
      />

      {error && <div className="error">{error}</div>}

      <section>
        <table className="table">
          <thead>
            <tr>
              <th>{t("shell.logs.kind", "Kind")}</th>
              <th>{t("shell.logs.level", "Level")}</th>
              <th>{t("shell.logs.title", "Title")}</th>
              <th>{t("shell.logs.summary", "Summary")}</th>
              <th>{t("shell.logs.scope", "Scope")}</th>
              <th>{t("shell.logs.at", "At")}</th>
            </tr>
          </thead>
          <tbody>
            {logs.map((log) => (
              <tr key={log.id}>
                <td>{log.kind}</td>
                <td><span className={`pill pill--${log.level}`}>{log.level}</span></td>
                <td>{log.title}</td>
                <td>{log.summary}</td>
                <td>
                  <code>{log.run_id ?? "—"}</code>
                  {log.task_id ? <><br /><code>{log.task_id}</code></> : null}
                </td>
                <td>{formatDateTime(log.at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
