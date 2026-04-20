import { useEffect, useState } from "react";

import { listLogs, type SystemLog } from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useUiHelpers } from "@/stores/ui";

export function LogsPage() {
  const { t, formatDateTime } = useUiHelpers();
  const [logs, setLogs] = useState<SystemLog[]>([]);

  useEffect(() => {
    void listLogs().then(setLogs);
  }, []);

  return (
    <div className="page">
      <PageHeader
        title={t("shell.logs.page_title", "日志")}
        description={t(
          "shell.logs.page_description",
          "这里只保留系统级、扩展级和任务级的总览日志；业务过程日志则贴近各自详情页展示。",
        )}
      />

      <section className="surface-panel">
        <div className="stack-list">
          {logs.map((log) => (
            <article key={log.id} className="thread-card">
              <div>
                <div className="thread-card__title">{log.title}</div>
                <p>
                  {log.kind} · {log.level}
                </p>
                <span>{formatDateTime(log.at)}</span>
              </div>
              <span>{log.summary}</span>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
