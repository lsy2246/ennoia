import { useEffect, useMemo, useState } from "react";

import {
  listExtensionEvents,
  listLogs,
  listRuns,
  type ExecutionRun,
  type ExtensionRuntimeEvent,
  type SystemLog,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

type UnifiedLogItem = {
  id: string;
  at: string;
  level: string;
  source: string;
  kind: string;
  title: string;
  summary: string;
  details?: string | null;
};

function toRunLog(run: ExecutionRun): UnifiedLogItem {
  return {
    id: `run:${run.id}`,
    at: run.updated_at,
    level: "info",
    source: "run",
    kind: "run",
    title: run.goal || run.trigger,
    summary: `${run.stage} · ${run.conversation_id}`,
    details: run.id,
  };
}

function toExtensionEventLog(event: ExtensionRuntimeEvent): UnifiedLogItem {
  return {
    id: `event:${event.event_id}`,
    at: event.occurred_at,
    level: event.health === "Failed" ? "error" : event.health === "Degraded" ? "warn" : "info",
    source: event.extension_id ?? "extension",
    kind: "extension_event",
    title: event.event,
    summary: event.summary,
    details: event.diagnostics.map((item) => `${item.level}: ${item.summary}`).join("\n"),
  };
}

function toSystemLog(log: SystemLog): UnifiedLogItem {
  return {
    id: log.id,
    at: log.at,
    level: log.level,
    source: log.source,
    kind: log.kind,
    title: log.title,
    summary: log.summary,
    details: log.details,
  };
}

export function LogsPage() {
  const { formatDateTime, t } = useUiHelpers();
  const [items, setItems] = useState<UnifiedLogItem[]>([]);
  const [q, setQ] = useState("");
  const [level, setLevel] = useState("");
  const [source, setSource] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  const sources = useMemo(() => [...new Set(items.map((item) => item.source))].sort(), [items]);

  async function refresh() {
    setError(null);
    try {
      const [logs, extensionEvents, runs] = await Promise.all([
        listLogs(150),
        listExtensionEvents(80),
        listRuns(),
      ]);
      const next = [
        ...logs.map(toSystemLog),
        ...extensionEvents.map(toExtensionEventLog),
        ...runs.slice(0, 50).map(toRunLog),
      ].sort((left, right) => String(right.at).localeCompare(String(left.at)));
      setItems(next);
    } catch (err) {
      setError(String(err));
    }
  }

  const filtered = items.filter((item) => {
    if (level && item.level !== level) {
      return false;
    }
    if (source && item.source !== source) {
      return false;
    }
    if (!q.trim()) {
      return true;
    }
    const haystack = [item.title, item.summary, item.details, item.kind, item.source]
      .filter(Boolean)
      .join("\n")
      .toLowerCase();
    return haystack.includes(q.trim().toLowerCase());
  });

  return (
    <div className="logs-layout">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.logs.eyebrow", "统一日志")}</span>
          <h1>{t("web.logs.title", "前端、后端、扩展事件和运行摘要在同一条流里。")}</h1>
          <p>{t("web.logs.description", "这里就是统一观测台。按来源、等级、类型和关键词筛选，不再拆独立 Observatory。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="filter-bar">
          <input value={q} onChange={(event) => setQ(event.target.value)} placeholder={t("web.logs.search_placeholder", "搜索标题、摘要、详情或来源")} />
          <select value={level} onChange={(event) => setLevel(event.target.value)}>
            <option value="">{t("web.logs.all_levels", "全部等级")}</option>
            <option value="info">info</option>
            <option value="warn">warn</option>
            <option value="error">error</option>
          </select>
          <select value={source} onChange={(event) => setSource(event.target.value)}>
            <option value="">{t("web.logs.all_sources", "全部来源")}</option>
            {sources.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
          <button type="button" onClick={() => void refresh()}>{t("web.action.refresh", "刷新")}</button>
        </div>
        <div className="stack">
          {filtered.map((item) => (
            <article key={item.id} className={`log-card log-card--${item.level}`}>
              <header>
                <strong>{item.title}</strong>
                <span>{item.level} · {item.source} / {item.kind}</span>
              </header>
              <p>{item.summary}</p>
              {item.details ? <pre>{item.details}</pre> : null}
              <small>{formatDateTime(item.at)} · {item.id}</small>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
