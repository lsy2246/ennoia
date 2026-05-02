import { useEffect, useMemo, useState } from "react";

import {
  ApiError,
  createLogsStream,
  getLogsOverview,
  parseLogsStreamPayload,
  getLogTraceDetail,
  listLogEntries,
  listLogTraces,
  type LogEntry,
  type LogsOverview,
  type LogTraceDetail,
  type LogTraceRecord,
} from "@ennoia/api-client";
import { MultiSelect, type MultiSelectOption } from "@/components/MultiSelect";
import { StatusNotice } from "@/components/StatusNotice";
import { useUiHelpers } from "@/stores/ui";

type UnifiedFilters = {
  q: string;
  scopes: Array<"error" | "warn" | "slow">;
  signalTypes: Array<"log" | "trace">;
  components: string[];
  sourceKinds: string[];
  requestId: string;
  traceId: string;
  logLevels: string[];
  traceStatuses: string[];
};

type TraceSummary = {
  traceId: string;
  requestId: string;
  name: string;
  component: string;
  kind: string;
  sourceKind: string;
  sourceId?: string | null;
  status: string;
  startedAt: string;
  endedAt: string;
  durationMs: number;
  spanCount: number;
  lastSeq: number;
};

type DiagnosticFeedItem = {
  key: string;
  kind: "log" | "trace";
  timestamp: string;
  title: string;
  summary: string;
  component: string;
  sourceKind: string;
  sourceId?: string | null;
  requestId?: string | null;
  traceId?: string | null;
  durationMs?: number;
  badgeValue: string;
  badgeClass: string;
  priority: "error" | "warn" | "slow" | "normal";
  log?: LogEntry;
  trace?: TraceSummary;
};

const INITIAL_FILTERS: UnifiedFilters = {
  q: "",
  scopes: [],
  signalTypes: [],
  components: [],
  sourceKinds: [],
  requestId: "",
  traceId: "",
  logLevels: [],
  traceStatuses: [],
};

const SLOW_TRACE_THRESHOLD_MS = 1200;
const LOGS_ROUTE_PREFIX = "/api/logs";

function stringifyJson(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function levelBadgeClass(level: string) {
  switch (level.toLowerCase()) {
    case "error":
    case "fatal":
      return "badge--danger";
    case "warn":
    case "warning":
      return "badge--warn";
    case "info":
    case "debug":
    case "trace":
      return "badge--muted";
    default:
      return "badge--muted";
  }
}

function statusBadgeClass(status: string) {
  const normalized = status.toLowerCase();
  if (normalized.includes("error") || normalized.includes("fail")) {
    return "badge--danger";
  }
  if (normalized.includes("warn") || normalized.includes("timeout") || normalized.includes("cancel")) {
    return "badge--warn";
  }
  if (normalized.includes("ok") || normalized.includes("success") || normalized.includes("done")) {
    return "badge--success";
  }
  return "badge--muted";
}

function localizeLogLevel(level: string, t: (key: string, fallback: string) => string) {
  switch (level.toLowerCase()) {
    case "fatal":
      return t("web.logs_page.level.fatal", "致命");
    case "error":
      return t("web.logs_page.level.error", "错误");
    case "warn":
    case "warning":
      return t("web.logs_page.level.warn", "警告");
    case "info":
      return t("web.logs_page.level.info", "信息");
    case "debug":
      return t("web.logs_page.level.debug", "调试");
    case "trace":
      return t("web.logs_page.level.trace", "跟踪");
    default:
      return level;
  }
}

function localizeTraceStatus(status: string, t: (key: string, fallback: string) => string) {
  switch (status.toLowerCase()) {
    case "slow":
      return t("web.logs_page.status.slow", "慢");
    case "error":
    case "fail":
    case "failed":
      return t("web.logs_page.status.error", "错误");
    case "warn":
    case "warning":
      return t("web.logs_page.status.warn", "警告");
    case "timeout":
      return t("web.logs_page.status.timeout", "超时");
    case "cancel":
    case "cancelled":
      return t("web.logs_page.status.cancel", "已取消");
    case "ok":
    case "success":
    case "done":
      return t("web.logs_page.status.ok", "正常");
    default:
      return status;
  }
}

function localizeSignalType(kind: "log" | "trace", t: (key: string, fallback: string) => string) {
  return kind === "log"
    ? t("web.logs_page.kind.log", "日志")
    : t("web.logs_page.kind.trace", "链路");
}

function localizeSourceKind(sourceKind: string, t: (key: string, fallback: string) => string) {
  switch (sourceKind.toLowerCase()) {
    case "system":
      return t("web.logs_page.source.system", "系统");
    case "extension":
      return t("web.logs_page.source.extension", "扩展");
    case "route":
      return t("web.logs_page.source.route", "路由");
    case "permission":
      return t("web.logs_page.source.permission", "权限");
    case "action":
      return t("web.logs_page.source.action", "动作");
    case "interface":
      return t("web.logs_page.source.interface", "接口");
    case "hook":
      return t("web.logs_page.source.hook", "钩子");
    case "conversation":
      return t("web.logs_page.source.conversation", "会话");
    default:
      return sourceKind;
  }
}

function collectOptionValues(values: Array<string | null | undefined>) {
  return [...new Set(values.filter((value): value is string => Boolean(value && value.trim())))]
    .sort((left, right) => left.localeCompare(right));
}

function isLogsSelfRequest(path?: string | null) {
  return typeof path === "string" && path.startsWith(LOGS_ROUTE_PREFIX);
}

function isLogsNoiseLog(item: LogEntry) {
  return isLogsSelfRequest(item.source_id);
}

function isLogsNoiseTrace(item: LogTraceRecord) {
  return isLogsSelfRequest(item.source_id);
}

function mergeLogEntries(
  current: LogEntry[],
  incoming: LogEntry[],
) {
  const records = new Map(current.map((item) => [item.id, item]));
  for (const item of incoming) {
    records.set(item.id, item);
  }
  return [...records.values()]
    .sort((left, right) => right.seq - left.seq)
    .slice(0, 160);
}

function mergeLogTraces(
  current: LogTraceRecord[],
  incoming: LogTraceRecord[],
) {
  const records = new Map(current.map((item) => [item.id, item]));
  for (const item of incoming) {
    records.set(item.id, item);
  }
  return [...records.values()]
    .sort((left, right) => right.seq - left.seq)
    .slice(0, 200);
}

function buildTraceSummaries(spans: LogTraceRecord[]): TraceSummary[] {
  const grouped = new Map<string, LogTraceRecord[]>();
  for (const span of spans) {
    const bucket = grouped.get(span.trace_id);
    if (bucket) {
      bucket.push(span);
    } else {
      grouped.set(span.trace_id, [span]);
    }
  }

  return [...grouped.entries()]
    .map(([traceId, records]) => {
      const ordered = [...records].sort((left, right) => left.seq - right.seq);
      const root = ordered.find((record) => !record.parent_span_id) ?? ordered[0];
      const latest = ordered[ordered.length - 1];
      const startedAt = [...ordered]
        .map((record) => record.started_at)
        .sort((left, right) => left.localeCompare(right))[0] ?? root.started_at;
      const endedAt = [...ordered]
        .map((record) => record.ended_at)
        .sort((left, right) => right.localeCompare(left))[0] ?? latest.ended_at;

      return {
        traceId,
        requestId: root.request_id,
        name: root.name,
        component: root.component,
        kind: root.kind,
        sourceKind: root.source_kind,
        sourceId: root.source_id,
        status: ordered.some((record) => statusBadgeClass(record.status) === "badge--danger")
          ? "error"
          : root.status,
        startedAt,
        endedAt,
        durationMs: root.duration_ms,
        spanCount: ordered.length,
        lastSeq: latest.seq,
      };
    })
    .sort((left, right) => right.lastSeq - left.lastSeq);
}

function buildDiagnosticFeed(logs: LogEntry[], traces: TraceSummary[]): DiagnosticFeedItem[] {
  const logItems = logs.map<DiagnosticFeedItem>((item) => {
    const badgeClass = levelBadgeClass(item.level);
    const priority = badgeClass === "badge--danger"
      ? "error"
      : badgeClass === "badge--warn"
        ? "warn"
        : "normal";
    return {
      key: `log:${item.id}`,
      kind: "log",
      timestamp: item.created_at,
      title: item.event,
      summary: item.message,
      component: item.component,
      sourceKind: item.source_kind,
      sourceId: item.source_id,
      requestId: item.request_id,
      traceId: item.trace_id,
      badgeValue: item.level,
      badgeClass,
      priority,
      log: item,
    };
  });

  const traceItems = traces.map<DiagnosticFeedItem>((item) => {
    const statusClass = statusBadgeClass(item.status);
    const isSlow = item.durationMs >= SLOW_TRACE_THRESHOLD_MS;
    const priority = statusClass === "badge--danger"
      ? "error"
      : statusClass === "badge--warn"
        ? "warn"
        : isSlow
          ? "slow"
          : "normal";
    return {
      key: `trace:${item.traceId}`,
      kind: "trace",
      timestamp: item.endedAt,
      title: item.name,
      summary: `${item.spanCount} spans · ${item.durationMs} ms · ${item.traceId}`,
      component: item.component,
      sourceKind: item.sourceKind,
      sourceId: item.sourceId,
      requestId: item.requestId,
      traceId: item.traceId,
      durationMs: item.durationMs,
      badgeValue: isSlow && statusClass === "badge--muted" ? "slow" : item.status,
      badgeClass: isSlow && statusClass === "badge--muted" ? "badge--warn" : statusClass,
      priority,
      trace: item,
    };
  });

  return [...logItems, ...traceItems].sort((left, right) => right.timestamp.localeCompare(left.timestamp));
}

export function LogsPage() {
  const { formatDateTime, t } = useUiHelpers();
  const [overview, setOverview] = useState<LogsOverview | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [traceSpans, setTraceSpans] = useState<LogTraceRecord[]>([]);
  const [filters, setFilters] = useState<UnifiedFilters>(INITIAL_FILTERS);
  const [selectedItemKey, setSelectedItemKey] = useState<string | null>(null);
  const [selectedTrace, setSelectedTrace] = useState<LogTraceDetail | null>(null);
  const [busy, setBusy] = useState(false);
  const [loadingTraceDetail, setLoadingTraceDetail] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const streamDisconnectedMessage = t("web.logs.stream_disconnected", "日志流连接中断，正在等待自动重连。");

  const traceSummaries = useMemo(() => buildTraceSummaries(traceSpans), [traceSpans]);
  const feed = useMemo(() => buildDiagnosticFeed(logs, traceSummaries), [logs, traceSummaries]);

  const componentOptions = useMemo(
    () => collectOptionValues([...logs.map((item) => item.component), ...traceSummaries.map((item) => item.component)]),
    [logs, traceSummaries],
  );
  const sourceKindOptions = useMemo(
    () => collectOptionValues([...logs.map((item) => item.source_kind), ...traceSummaries.map((item) => item.sourceKind)]),
    [logs, traceSummaries],
  );
  const logLevelOptions = useMemo(() => collectOptionValues(logs.map((item) => item.level)), [logs]);
  const traceStatusOptions = useMemo(() => collectOptionValues(traceSummaries.map((item) => item.status)), [traceSummaries]);
  const scopeFilterOptions = useMemo<MultiSelectOption[]>(
    () => [
      { value: "error", label: t("web.logs_page.filters.scope_error", "异常") },
      { value: "warn", label: t("web.logs_page.filters.scope_warn", "告警") },
      { value: "slow", label: t("web.logs_page.filters.scope_slow", "慢链路") },
    ],
    [t],
  );
  const signalTypeFilterOptions = useMemo<MultiSelectOption[]>(
    () => [
      { value: "log", label: t("web.logs_page.filters.signal_type_log", "日志") },
      { value: "trace", label: t("web.logs_page.filters.signal_type_trace", "链路") },
    ],
    [t],
  );
  const componentFilterOptions = useMemo<MultiSelectOption[]>(
    () => componentOptions.map((item) => ({ value: item, label: item })),
    [componentOptions],
  );
  const sourceKindFilterOptions = useMemo<MultiSelectOption[]>(
    () => sourceKindOptions.map((item) => ({ value: item, label: localizeSourceKind(item, t) })),
    [sourceKindOptions, t],
  );
  const logLevelFilterOptions = useMemo<MultiSelectOption[]>(
    () => logLevelOptions.map((item) => ({ value: item, label: localizeLogLevel(item, t) })),
    [logLevelOptions, t],
  );
  const traceStatusFilterOptions = useMemo<MultiSelectOption[]>(
    () => traceStatusOptions.map((item) => ({ value: item, label: localizeTraceStatus(item, t) })),
    [traceStatusOptions, t],
  );

  const filteredFeed = useMemo(() => {
    return feed.filter((item) => {
      if (filters.scopes.length > 0) {
        const matchesSelectedScope = filters.scopes.some((scope) => {
          if (scope === "error") {
            return item.priority === "error";
          }
          if (scope === "warn") {
            return item.priority === "warn";
          }
          return item.kind === "trace" && (item.durationMs ?? 0) >= SLOW_TRACE_THRESHOLD_MS;
        });
        if (!matchesSelectedScope) {
          return false;
        }
      }
      if (filters.signalTypes.length > 0 && !filters.signalTypes.includes(item.kind)) {
        return false;
      }
      if (filters.components.length > 0 && !filters.components.includes(item.component)) {
        return false;
      }
      if (filters.sourceKinds.length > 0 && !filters.sourceKinds.includes(item.sourceKind)) {
        return false;
      }
      if (filters.requestId && item.requestId !== filters.requestId.trim()) {
        return false;
      }
      if (filters.traceId && item.traceId !== filters.traceId.trim()) {
        return false;
      }
      if (filters.logLevels.length > 0) {
        if (item.kind !== "log" || !item.log || !filters.logLevels.includes(item.log.level)) {
          return false;
        }
      }
      if (filters.traceStatuses.length > 0) {
        if (item.kind !== "trace" || !item.trace || !filters.traceStatuses.includes(item.trace.status)) {
          return false;
        }
      }
      if (!filters.q.trim()) {
        return true;
      }
      const haystack = [
        item.title,
        item.summary,
        item.component,
        item.sourceKind,
        item.requestId,
        item.traceId,
        item.kind === "log" ? stringifyJson(item.log?.attributes) : stringifyJson(item.trace),
      ]
        .filter(Boolean)
        .join("\n")
        .toLowerCase();
      return haystack.includes(filters.q.trim().toLowerCase());
    });
  }, [feed, filters]);

  const selectedItem = useMemo(
    () => filteredFeed.find((item) => item.key === selectedItemKey) ?? null,
    [filteredFeed, selectedItemKey],
  );

  const selectedTraceId = selectedItem?.kind === "trace"
    ? selectedItem.trace?.traceId ?? null
    : selectedItem?.log?.trace_id ?? null;

  const selectedTraceSummary = useMemo(
    () => (selectedTraceId ? traceSummaries.find((item) => item.traceId === selectedTraceId) ?? null : null),
    [selectedTraceId, traceSummaries],
  );

  const relatedLogs = useMemo(() => {
    if (!selectedItem) {
      return [];
    }
    const scoped = logs.filter((item) => {
      if (selectedTraceId && item.trace_id === selectedTraceId) {
        return true;
      }
      return Boolean(selectedItem.requestId && item.request_id === selectedItem.requestId);
    });
    return scoped
      .filter((item) => item.id !== selectedItem.log?.id)
      .sort((left, right) => right.seq - left.seq)
      .slice(0, 6);
  }, [logs, selectedItem, selectedTraceId]);

  const sortedTraceSpans = useMemo(() => {
    if (!selectedTrace) {
      return [];
    }
    return [...selectedTrace.spans].sort((left, right) => left.seq - right.seq);
  }, [selectedTrace]);

  const previewTraceSpans = useMemo(() => {
    if (!selectedItem) {
      return [];
    }
    if (selectedItem.kind === "trace") {
      return sortedTraceSpans;
    }
    return sortedTraceSpans.slice(0, 4);
  }, [selectedItem, sortedTraceSpans]);

  const issueLogCount = useMemo(
    () => logs.filter((item) => {
      const badge = levelBadgeClass(item.level);
      return badge === "badge--danger" || badge === "badge--warn";
    }).length,
    [logs],
  );
  const issueTraceCount = useMemo(
    () => traceSummaries.filter((item) => {
      const badge = statusBadgeClass(item.status);
      return badge === "badge--danger" || badge === "badge--warn";
    }).length,
    [traceSummaries],
  );
  const slowTraceCount = useMemo(
    () => traceSummaries.filter((item) => item.durationMs >= SLOW_TRACE_THRESHOLD_MS).length,
    [traceSummaries],
  );
  const activeRequestCount = useMemo(
    () => new Set(
      [...logs.map((item) => item.request_id), ...traceSummaries.map((item) => item.requestId)]
        .filter((item): item is string => Boolean(item && item.trim())),
    ).size,
    [logs, traceSummaries],
  );

  async function refresh() {
    setBusy(true);
    setError(null);
    try {
      const [nextOverview, nextLogs, nextTraces] = await Promise.all([
        getLogsOverview(),
        listLogEntries({ limit: 160 }),
        listLogTraces({ limit: 200 }),
      ]);
      const visibleLogs = nextLogs.filter((item) => !isLogsNoiseLog(item));
      const visibleTraces = nextTraces.filter((item) => !isLogsNoiseTrace(item));
      const nextTraceSummaries = buildTraceSummaries(visibleTraces);
      const nextFeed = buildDiagnosticFeed(visibleLogs, nextTraceSummaries);
      setOverview(nextOverview);
      setLogs(visibleLogs);
      setTraceSpans(visibleTraces);
      setSelectedItemKey((current) => nextFeed.some((item) => item.key === current) ? current : nextFeed[0]?.key ?? null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  useEffect(() => {
    const stream = createLogsStream();
    stream.addEventListener("logs.delta", (event) => {
      if (!(event instanceof MessageEvent) || typeof event.data !== "string") {
        return;
      }
      try {
        const payload = parseLogsStreamPayload(event.data);
        const visibleLogs = payload.logs.filter((item) => !isLogsNoiseLog(item));
        const visibleTraces = payload.traces.filter((item) => !isLogsNoiseTrace(item));
        setOverview(payload.overview);
        if (visibleLogs.length > 0) {
          setLogs((current) => mergeLogEntries(current, visibleLogs));
        }
        if (visibleTraces.length > 0) {
          setTraceSpans((current) => mergeLogTraces(current, visibleTraces));
        }
      } catch (err) {
        setError(String(err));
      }
    });
    stream.addEventListener("logs.error", (event) => {
      if (event instanceof MessageEvent && typeof event.data === "string" && event.data.trim()) {
        setError(event.data);
      }
    });
    stream.onerror = () => {
      setError(streamDisconnectedMessage);
    };
    stream.onopen = () => {
      setError((current) =>
        current === streamDisconnectedMessage
          ? null
          : current,
      );
    };
    return () => stream.close();
  }, [streamDisconnectedMessage]);

  useEffect(() => {
    setSelectedItemKey((current) =>
      filteredFeed.some((item) => item.key === current) ? current : filteredFeed[0]?.key ?? null);
  }, [filteredFeed]);

  useEffect(() => {
    if (!selectedTraceId) {
      setSelectedTrace(null);
      return;
    }

    let cancelled = false;
    setLoadingTraceDetail(true);
    void getLogTraceDetail(selectedTraceId)
      .then((detail) => {
        if (!cancelled) {
          setSelectedTrace(detail);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          if (err instanceof ApiError && err.status === 404) {
            setSelectedTrace(null);
            return;
          }
          setError(String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingTraceDetail(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedTraceId]);

  return (
    <div className="logs-layout">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className="work-panel logs-header-card">
        <div className="logs-toolbar">
          <div className="page-heading">
            <span>{t("web.logs_page.eyebrow", "Logs")}</span>
            <h1>{t("web.logs_page.title", "统一诊断最近异常、日志与链路上下文。")}</h1>
            <p>{t("web.logs_page.description", "先定位异常与慢链路，再按 request 或 trace 深挖上下文，不再把日志和日志拆成两套页面心智。")}</p>
          </div>
          <button type="button" className="secondary" onClick={() => void refresh()} disabled={busy}>
            {busy ? t("web.common.loading", "加载中…") : t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className="logs-summary-grid">
          <article className="metric-card logs-metric-card">
            <span>{t("web.logs_page.metrics.issue_logs", "异常日志")}</span>
            <strong>{issueLogCount}</strong>
            <small>{overview ? `${t("web.logs_page.metrics.total_label", "总量")} ${overview.log_count}` : "—"}</small>
          </article>
          <article className="metric-card logs-metric-card">
            <span>{t("web.logs_page.metrics.issue_traces", "异常链路")}</span>
            <strong>{issueTraceCount}</strong>
            <small>{overview ? `${t("web.logs_page.metrics.total_label", "总量")} ${overview.trace_count}` : "—"}</small>
          </article>
          <article className="metric-card logs-metric-card">
            <span>{t("web.logs_page.metrics.slow_traces", "慢链路")}</span>
            <strong>{slowTraceCount}</strong>
            <small>{`>${SLOW_TRACE_THRESHOLD_MS} ms`}</small>
          </article>
          <article className="metric-card logs-metric-card">
            <span>{t("web.logs_page.metrics.active_requests", "活跃 request")}</span>
            <strong>{activeRequestCount}</strong>
            <small>{overview ? `${overview.span_count} ${t("web.logs_page.metrics.spans_suffix", "spans")}` : "—"}</small>
          </article>
        </div>
      </section>

      <section className="work-panel logs-diagnostic-grid logs-workbench">
        <div className="logs-panel logs-panel--feed logs-column">
          <div className="logs-section-header">
            <div className="page-heading">
              <span>{t("web.logs_page.feed.eyebrow", "Diagnostic feed")}</span>
              <h1>{t("web.logs_page.feed.title", "统一诊断流")}</h1>
              <p>{t("web.logs_page.feed.description", "列表按时间统一展示事件；需要收窄范围时，展开高级筛选按类型、严重程度或 ID 精确定位。")}</p>
            </div>
            <div className="logs-feed-count">
              {`${filteredFeed.length} ${t("web.logs_page.feed.count", "条")}`}
            </div>
          </div>

          <div className="logs-feed-tools">
            <input
              className="logs-search"
              value={filters.q}
              onChange={(event) => setFilters((current) => ({ ...current, q: event.target.value }))}
              placeholder={t("web.logs_page.feed.search", "搜索消息、事件、trace_id、request_id 或组件")}
            />
            <details className="logs-filter-popover">
              <summary className="secondary">{t("web.logs_page.filters.title", "高级筛选")}</summary>
              <div className="logs-filter-popover__panel">
                <div className="logs-filter-grid">
                  <label className="logs-filter-field">
                    <span>{t("web.logs_page.filters.scope", "范围")}</span>
                    <MultiSelect
                      values={filters.scopes}
                      onChange={(values) => setFilters((current) => ({ ...current, scopes: values as UnifiedFilters["scopes"] }))}
                      options={scopeFilterOptions}
                      placeholder={t("web.logs_page.filters.scope_all", "全部范围")}
                    />
                  </label>
                  <label className="logs-filter-field">
                    <span>{t("web.logs_page.filters.signal_type", "类型")}</span>
                    <MultiSelect
                      values={filters.signalTypes}
                      onChange={(values) => setFilters((current) => ({ ...current, signalTypes: values as UnifiedFilters["signalTypes"] }))}
                      options={signalTypeFilterOptions}
                      placeholder={t("web.logs_page.filters.signal_type_all", "全部类型")}
                    />
                  </label>
                  <label className="logs-filter-field">
                    <span>{t("web.logs_page.filters.component", "组件")}</span>
                    <MultiSelect
                      values={filters.components}
                      onChange={(values) => setFilters((current) => ({ ...current, components: values }))}
                      options={componentFilterOptions}
                      placeholder={t("web.logs_page.filters.all", "全部")}
                    />
                  </label>
                  <label className="logs-filter-field">
                    <span>{t("web.logs_page.filters.source_kind", "来源")}</span>
                    <MultiSelect
                      values={filters.sourceKinds}
                      onChange={(values) => setFilters((current) => ({ ...current, sourceKinds: values }))}
                      options={sourceKindFilterOptions}
                      placeholder={t("web.logs_page.filters.all", "全部")}
                    />
                  </label>
                  <label className="logs-filter-field">
                    <span>{t("web.logs_page.filters.log_level", "日志等级")}</span>
                    <MultiSelect
                      values={filters.logLevels}
                      onChange={(values) => setFilters((current) => ({ ...current, logLevels: values }))}
                      options={logLevelFilterOptions}
                      placeholder={t("web.logs_page.filters.all", "全部")}
                    />
                  </label>
                  <label className="logs-filter-field">
                    <span>{t("web.logs_page.filters.trace_status", "链路结果")}</span>
                    <MultiSelect
                      values={filters.traceStatuses}
                      onChange={(values) => setFilters((current) => ({ ...current, traceStatuses: values }))}
                      options={traceStatusFilterOptions}
                      placeholder={t("web.logs_page.filters.all", "全部")}
                    />
                  </label>
                  <label className="logs-filter-field">
                    <span>{t("web.logs_page.filters.request_id", "请求编号")}</span>
                    <input
                      value={filters.requestId}
                      onChange={(event) => setFilters((current) => ({ ...current, requestId: event.target.value }))}
                      placeholder={t("web.logs_page.filters.request_id_placeholder", "输入 request_id")}
                    />
                  </label>
                  <label className="logs-filter-field">
                    <span>{t("web.logs_page.filters.trace_id", "链路编号")}</span>
                    <input
                      value={filters.traceId}
                      onChange={(event) => setFilters((current) => ({ ...current, traceId: event.target.value }))}
                      placeholder={t("web.logs_page.filters.trace_id_placeholder", "输入 trace_id")}
                    />
                  </label>
                </div>
              </div>
            </details>
          </div>

          <div className="logs-scroll stack">
            {filteredFeed.length === 0 ? (
              <div className="empty-card">{t("web.logs_page.feed.empty", "当前筛选下没有可显示的诊断事件。")}</div>
            ) : (
              filteredFeed.map((item) => (
                <article
                  key={item.key}
                  className={`resource-card logs-feed-card logs-feed-card--${item.priority} ${selectedItemKey === item.key ? "logs-item--active" : ""}`}
                >
                  <button type="button" className="plain-card-button" onClick={() => setSelectedItemKey(item.key)}>
                    <header className="logs-feed-card__header">
                      <div className="stack">
                        <strong>{item.title}</strong>
                        <span className="logs-feed-card__kind">
                          {localizeSignalType(item.kind, t)}
                        </span>
                      </div>
                      <span className="logs-meta">
                        <span className={`badge ${item.badgeClass}`}>
                          {item.kind === "log"
                            ? localizeLogLevel(item.badgeValue, t)
                            : localizeTraceStatus(item.badgeValue, t)}
                        </span>
                        <span className="badge badge--muted">{item.component}</span>
                      </span>
                    </header>
                    <p>{item.summary}</p>
                    <div className="logs-inline-meta">
                      <span>{formatDateTime(item.timestamp)}</span>
                      {item.requestId ? <span>request:{item.requestId}</span> : null}
                      {item.traceId ? <span>trace:{item.traceId}</span> : null}
                    </div>
                  </button>
                </article>
              ))
            )}
          </div>
        </div>

        <div className="logs-panel logs-panel--detail logs-column">
          <div className="logs-section-header">
            <div className="page-heading">
              <span>{t("web.logs_page.detail.eyebrow", "Context")}</span>
              <h1>{t("web.logs_page.detail.title", "关联上下文")}</h1>
              <p>{t("web.logs_page.detail.description", "这里统一展示选中事件的关键信息、关联日志和 trace 链路。")}</p>
            </div>
            {loadingTraceDetail ? <span>{t("web.common.loading", "加载中…")}</span> : null}
          </div>

          <div className="details-panel logs-detail">
            {selectedItem ? (
              <div className="stack">
                <section className="logs-detail-block">
                  <div className="logs-detail-block__header">
                    <strong>{t("web.logs_page.detail.summary", "概览")}</strong>
                  </div>
                  <div className="kv-list logs-kv-list">
                    <span>{t("web.logs_page.detail.signal", "类型")}</span>
                    <strong>{localizeSignalType(selectedItem.kind, t)}</strong>
                    <span>{t("web.logs_page.detail.time", "时间")}</span>
                    <strong>{formatDateTime(selectedItem.timestamp)}</strong>
                    <span>{t("web.logs_page.detail.component", "组件")}</span>
                    <strong>{selectedItem.component}</strong>
                    <span>{t("web.logs_page.detail.source", "来源")}</span>
                    <strong>{localizeSourceKind(selectedItem.sourceKind, t)}</strong>
                    <span>{t("web.logs_page.filters.request_id", "请求编号")}</span>
                    <strong>{selectedItem.requestId ?? t("web.common.none", "无")}</strong>
                    <span>{t("web.logs_page.filters.trace_id", "链路编号")}</span>
                    <strong>{selectedItem.traceId ?? t("web.common.none", "无")}</strong>
                  </div>
                </section>

                {selectedItem.kind === "log" && selectedItem.log ? (
                  <section className="logs-detail-block">
                    <div className="logs-detail-block__header">
                      <strong>{t("web.logs_page.detail.core", "核心信息")}</strong>
                    </div>
                    <div className="mini-card logs-context-card">
                      <strong>{selectedItem.log.event}</strong>
                      <p>{selectedItem.log.message}</p>
                      <div className="logs-meta">
                        <span className={`badge ${levelBadgeClass(selectedItem.log.level)}`}>{localizeLogLevel(selectedItem.log.level, t)}</span>
                        {selectedItem.log.span_id ? <span className="badge badge--muted">span:{selectedItem.log.span_id}</span> : null}
                      </div>
                    </div>
                  </section>
                ) : null}

                {selectedItem.kind === "trace" && selectedItem.trace ? (
                  <section className="logs-detail-block">
                    <div className="logs-detail-block__header">
                      <strong>{t("web.logs_page.detail.core", "核心信息")}</strong>
                    </div>
                    <div className="mini-card logs-context-card">
                      <strong>{selectedItem.trace.name}</strong>
                      <p>{`${selectedItem.trace.spanCount} ${t("web.logs_page.metrics.spans_suffix", "spans")} · ${selectedItem.trace.durationMs} ms`}</p>
                      <div className="logs-meta">
                        <span className={`badge ${statusBadgeClass(selectedItem.trace.status)}`}>{localizeTraceStatus(selectedItem.trace.status, t)}</span>
                        <span className="badge badge--muted">{selectedItem.trace.kind}</span>
                        {selectedItem.trace.sourceId ? <span className="badge badge--muted">{selectedItem.trace.sourceId}</span> : null}
                      </div>
                    </div>
                    {selectedTraceSummary ? (
                      <div className="kv-list logs-kv-list">
                        <span>{t("web.logs_page.detail.duration", "耗时")}</span>
                        <strong>{`${selectedTraceSummary.durationMs} ms`}</strong>
                        <span>{t("web.logs_page.detail.spans", "Span 数")}</span>
                        <strong>{selectedTraceSummary.spanCount}</strong>
                        <span>{t("web.logs_page.detail.started_at", "开始时间")}</span>
                        <strong>{formatDateTime(selectedTraceSummary.startedAt)}</strong>
                        <span>{t("web.logs_page.detail.ended_at", "结束时间")}</span>
                        <strong>{formatDateTime(selectedTraceSummary.endedAt)}</strong>
                      </div>
                    ) : null}
                  </section>
                ) : null}

                {selectedTraceSummary ? (
                  <section className="logs-detail-block">
                    <div className="logs-detail-block__header">
                      <strong>{t("web.logs_page.detail.related_trace", "关联链路")}</strong>
                    </div>
                    <div className="mini-card logs-context-card">
                      <strong>{selectedTraceSummary.name}</strong>
                      <div className="logs-inline-meta">
                        <span>{selectedTraceSummary.traceId}</span>
                        <span>{`${selectedTraceSummary.spanCount} ${t("web.logs_page.metrics.spans_suffix", "spans")}`}</span>
                        <span>{`${selectedTraceSummary.durationMs} ms`}</span>
                      </div>
                    </div>

                    {previewTraceSpans.length > 0 ? (
                      <div className="timeline-list">
                        {previewTraceSpans.map((span) => (
                          <article key={span.id} className="timeline-item logs-timeline-item">
                            <div className="stack">
                              <strong>{span.duration_ms} ms</strong>
                              <small>{formatDateTime(span.started_at)}</small>
                            </div>
                            <div className="stack">
                              <div className="logs-section-header">
                                <strong>{span.name}</strong>
                                <span className="logs-meta">
                                  <span className={`badge ${statusBadgeClass(span.status)}`}>{localizeTraceStatus(span.status, t)}</span>
                                  <span className="badge badge--muted">{span.kind}</span>
                                </span>
                              </div>
                              <div className="logs-meta">
                                <span className="badge badge--muted">{span.component}</span>
                                <span className="badge badge--muted">{localizeSourceKind(span.source_kind, t)}</span>
                                {span.parent_span_id ? <span className="badge badge--muted">parent:{span.parent_span_id}</span> : null}
                              </div>
                            </div>
                          </article>
                        ))}
                      </div>
                    ) : null}

                    {selectedItem.kind === "log" && sortedTraceSpans.length > previewTraceSpans.length ? (
                      <details className="details-panel logs-detail-disclosure">
                        <summary>{t("web.logs_page.detail.expand_timeline", "展开完整链路")}</summary>
                        <div className="timeline-list">
                          {sortedTraceSpans.map((span) => (
                            <article key={span.id} className="timeline-item logs-timeline-item">
                              <div className="stack">
                                <strong>{span.duration_ms} ms</strong>
                                <small>{formatDateTime(span.started_at)}</small>
                              </div>
                              <div className="stack">
                                <div className="logs-section-header">
                                  <strong>{span.name}</strong>
                                  <span className="logs-meta">
                                    <span className={`badge ${statusBadgeClass(span.status)}`}>{localizeTraceStatus(span.status, t)}</span>
                                    <span className="badge badge--muted">{span.kind}</span>
                                  </span>
                                </div>
                                <div className="logs-meta">
                                  <span className="badge badge--muted">{span.component}</span>
                                  <span className="badge badge--muted">{localizeSourceKind(span.source_kind, t)}</span>
                                  {span.source_id ? <span className="badge badge--muted">{span.source_id}</span> : null}
                                  {span.parent_span_id ? <span className="badge badge--muted">parent:{span.parent_span_id}</span> : null}
                                </div>
                                <pre className="logs-json">{stringifyJson(span.attributes)}</pre>
                              </div>
                            </article>
                          ))}
                        </div>
                      </details>
                    ) : null}
                  </section>
                ) : null}

                <section className="logs-detail-block">
                  <div className="logs-detail-block__header">
                    <strong>{t("web.logs_page.detail.related_logs", "关联日志")}</strong>
                  </div>
                  <div className="logs-related-logs">
                    {selectedItem.kind === "log" && selectedItem.log ? (
                      <article className="log-card logs-related-log logs-related-log--current">
                        <header className="logs-related-log__header">
                          <div className="stack logs-related-log__title">
                            <small>{t("web.logs_page.detail.current_log", "当前日志")}</small>
                            <strong>{selectedItem.log.event}</strong>
                          </div>
                          <span className="logs-meta">
                            <span className={`badge ${levelBadgeClass(selectedItem.log.level)}`}>{localizeLogLevel(selectedItem.log.level, t)}</span>
                            <span className="badge badge--muted">{selectedItem.log.component}</span>
                          </span>
                        </header>
                        <p className="logs-related-log__message">{selectedItem.log.message}</p>
                        <div className="logs-inline-meta logs-related-log__meta">
                          <span>{formatDateTime(selectedItem.log.created_at)}</span>
                          {selectedItem.log.request_id ? <span>request:{selectedItem.log.request_id}</span> : null}
                          {selectedItem.log.trace_id ? <span>trace:{selectedItem.log.trace_id}</span> : null}
                          {selectedItem.log.span_id ? <span>span:{selectedItem.log.span_id}</span> : null}
                          <span>{localizeSourceKind(selectedItem.log.source_kind, t)}</span>
                        </div>
                      </article>
                    ) : null}

                    {relatedLogs.length > 0 ? (
                      relatedLogs.map((item) => (
                        <article key={item.id} className="log-card logs-related-log">
                          <header className="logs-related-log__header">
                            <div className="stack logs-related-log__title">
                              <strong>{item.event}</strong>
                              <small>{formatDateTime(item.created_at)}</small>
                            </div>
                            <span className="logs-meta">
                              <span className={`badge ${levelBadgeClass(item.level)}`}>{localizeLogLevel(item.level, t)}</span>
                              <span className="badge badge--muted">{item.component}</span>
                            </span>
                          </header>
                          <p className="logs-related-log__message">{item.message}</p>
                          <div className="logs-inline-meta logs-related-log__meta">
                            <span>{localizeSourceKind(item.source_kind, t)}</span>
                            {item.request_id ? <span>request:{item.request_id}</span> : null}
                            {item.trace_id ? <span>trace:{item.trace_id}</span> : null}
                            {item.span_id ? <span>span:{item.span_id}</span> : null}
                          </div>
                        </article>
                      ))
                    ) : (
                      <div className="empty-card logs-related-log-empty">
                        <strong>{t("web.logs_page.detail.related_logs_empty_title", "没有更多关联日志")}</strong>
                        <p>{t("web.logs_page.detail.related_logs_empty", "当前上下文里没有更多关联日志。")}</p>
                      </div>
                    )}
                  </div>
                </section>

                {selectedItem.kind === "log" && selectedItem.log ? (
                  <section className="logs-detail-block">
                    <div className="logs-detail-block__header">
                      <strong>{t("web.logs_page.common.attributes", "属性")}</strong>
                    </div>
                    <pre className="logs-json">{stringifyJson(selectedItem.log.attributes)}</pre>
                  </section>
                ) : null}
              </div>
            ) : (
              <div className="empty-card">{t("web.logs_page.detail.empty", "选择一条事件后在这里查看关联日志和链路。")}</div>
            )}
          </div>
        </div>
      </section>
    </div>
  );
}


