import { useEffect, useMemo, useState } from "react";

import {
  createObservabilityStream,
  ApiError,
  getObservabilityOverview,
  parseObservabilityStreamPayload,
  getObservabilityTraceDetail,
  listObservabilityLogs,
  listObservabilityTraces,
  type ObservationLogEntry,
  type ObservationOverview,
  type ObservationTraceDetail,
  type ObservationSpanRecord,
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
  log?: ObservationLogEntry;
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
const OBSERVABILITY_ROUTE_PREFIX = "/api/logs";

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
      return t("web.observability.level.fatal", "致命");
    case "error":
      return t("web.observability.level.error", "错误");
    case "warn":
    case "warning":
      return t("web.observability.level.warn", "警告");
    case "info":
      return t("web.observability.level.info", "信息");
    case "debug":
      return t("web.observability.level.debug", "调试");
    case "trace":
      return t("web.observability.level.trace", "跟踪");
    default:
      return level;
  }
}

function localizeTraceStatus(status: string, t: (key: string, fallback: string) => string) {
  switch (status.toLowerCase()) {
    case "slow":
      return t("web.observability.status.slow", "慢");
    case "error":
    case "fail":
    case "failed":
      return t("web.observability.status.error", "错误");
    case "warn":
    case "warning":
      return t("web.observability.status.warn", "警告");
    case "timeout":
      return t("web.observability.status.timeout", "超时");
    case "cancel":
    case "cancelled":
      return t("web.observability.status.cancel", "已取消");
    case "ok":
    case "success":
    case "done":
      return t("web.observability.status.ok", "正常");
    default:
      return status;
  }
}

function localizeSignalType(kind: "log" | "trace", t: (key: string, fallback: string) => string) {
  return kind === "log"
    ? t("web.observability.kind.log", "日志")
    : t("web.observability.kind.trace", "链路");
}

function localizeSourceKind(sourceKind: string, t: (key: string, fallback: string) => string) {
  switch (sourceKind.toLowerCase()) {
    case "system":
      return t("web.observability.source.system", "系统");
    case "extension":
      return t("web.observability.source.extension", "扩展");
    case "route":
      return t("web.observability.source.route", "路由");
    case "permission":
      return t("web.observability.source.permission", "权限");
    case "action":
      return t("web.observability.source.action", "动作");
    case "interface":
      return t("web.observability.source.interface", "接口");
    case "hook":
      return t("web.observability.source.hook", "钩子");
    case "conversation":
      return t("web.observability.source.conversation", "会话");
    default:
      return sourceKind;
  }
}

function collectOptionValues(values: Array<string | null | undefined>) {
  return [...new Set(values.filter((value): value is string => Boolean(value && value.trim())))]
    .sort((left, right) => left.localeCompare(right));
}

function isObservabilitySelfRequest(path?: string | null) {
  return typeof path === "string" && path.startsWith(OBSERVABILITY_ROUTE_PREFIX);
}

function isObservabilityNoiseLog(item: ObservationLogEntry) {
  return isObservabilitySelfRequest(item.source_id);
}

function isObservabilityNoiseTrace(item: ObservationSpanRecord) {
  return isObservabilitySelfRequest(item.source_id);
}

function mergeObservationLogs(
  current: ObservationLogEntry[],
  incoming: ObservationLogEntry[],
) {
  const records = new Map(current.map((item) => [item.id, item]));
  for (const item of incoming) {
    records.set(item.id, item);
  }
  return [...records.values()]
    .sort((left, right) => right.seq - left.seq)
    .slice(0, 160);
}

function mergeObservationSpans(
  current: ObservationSpanRecord[],
  incoming: ObservationSpanRecord[],
) {
  const records = new Map(current.map((item) => [item.id, item]));
  for (const item of incoming) {
    records.set(item.id, item);
  }
  return [...records.values()]
    .sort((left, right) => right.seq - left.seq)
    .slice(0, 200);
}

function buildTraceSummaries(spans: ObservationSpanRecord[]): TraceSummary[] {
  const grouped = new Map<string, ObservationSpanRecord[]>();
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

function buildDiagnosticFeed(logs: ObservationLogEntry[], traces: TraceSummary[]): DiagnosticFeedItem[] {
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

export function Observability() {
  const { formatDateTime, t } = useUiHelpers();
  const [overview, setOverview] = useState<ObservationOverview | null>(null);
  const [logs, setLogs] = useState<ObservationLogEntry[]>([]);
  const [traceSpans, setTraceSpans] = useState<ObservationSpanRecord[]>([]);
  const [filters, setFilters] = useState<UnifiedFilters>(INITIAL_FILTERS);
  const [selectedItemKey, setSelectedItemKey] = useState<string | null>(null);
  const [selectedTrace, setSelectedTrace] = useState<ObservationTraceDetail | null>(null);
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
      { value: "error", label: t("web.observability.filters.scope_error", "异常") },
      { value: "warn", label: t("web.observability.filters.scope_warn", "告警") },
      { value: "slow", label: t("web.observability.filters.scope_slow", "慢链路") },
    ],
    [t],
  );
  const signalTypeFilterOptions = useMemo<MultiSelectOption[]>(
    () => [
      { value: "log", label: t("web.observability.filters.signal_type_log", "日志") },
      { value: "trace", label: t("web.observability.filters.signal_type_trace", "链路") },
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
        getObservabilityOverview(),
        listObservabilityLogs({ limit: 160 }),
        listObservabilityTraces({ limit: 200 }),
      ]);
      const visibleLogs = nextLogs.filter((item) => !isObservabilityNoiseLog(item));
      const visibleTraces = nextTraces.filter((item) => !isObservabilityNoiseTrace(item));
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
    const stream = createObservabilityStream();
    stream.addEventListener("logs.delta", (event) => {
      if (!(event instanceof MessageEvent) || typeof event.data !== "string") {
        return;
      }
      try {
        const payload = parseObservabilityStreamPayload(event.data);
        const visibleLogs = payload.logs.filter((item) => !isObservabilityNoiseLog(item));
        const visibleTraces = payload.traces.filter((item) => !isObservabilityNoiseTrace(item));
        setOverview(payload.overview);
        if (visibleLogs.length > 0) {
          setLogs((current) => mergeObservationLogs(current, visibleLogs));
        }
        if (visibleTraces.length > 0) {
          setTraceSpans((current) => mergeObservationSpans(current, visibleTraces));
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
    void getObservabilityTraceDetail(selectedTraceId)
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
    <div className="observability-layout">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className="work-panel observability-header-card">
        <div className="observability-toolbar">
          <div className="page-heading">
            <span>{t("web.observability.eyebrow", "Observability")}</span>
            <h1>{t("web.observability.title", "统一诊断最近异常、日志与链路上下文。")}</h1>
            <p>{t("web.observability.description", "先定位异常与慢链路，再按 request 或 trace 深挖上下文，不再把日志和观测拆成两套页面心智。")}</p>
          </div>
          <button type="button" className="secondary" onClick={() => void refresh()} disabled={busy}>
            {busy ? t("web.common.loading", "加载中…") : t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className="observability-summary-grid">
          <article className="metric-card observability-metric-card">
            <span>{t("web.observability.metrics.issue_logs", "异常日志")}</span>
            <strong>{issueLogCount}</strong>
            <small>{overview ? `${t("web.observability.metrics.total_label", "总量")} ${overview.log_count}` : "—"}</small>
          </article>
          <article className="metric-card observability-metric-card">
            <span>{t("web.observability.metrics.issue_traces", "异常链路")}</span>
            <strong>{issueTraceCount}</strong>
            <small>{overview ? `${t("web.observability.metrics.total_label", "总量")} ${overview.trace_count}` : "—"}</small>
          </article>
          <article className="metric-card observability-metric-card">
            <span>{t("web.observability.metrics.slow_traces", "慢链路")}</span>
            <strong>{slowTraceCount}</strong>
            <small>{`>${SLOW_TRACE_THRESHOLD_MS} ms`}</small>
          </article>
          <article className="metric-card observability-metric-card">
            <span>{t("web.observability.metrics.active_requests", "活跃 request")}</span>
            <strong>{activeRequestCount}</strong>
            <small>{overview ? `${overview.span_count} ${t("web.observability.metrics.spans_suffix", "spans")}` : "—"}</small>
          </article>
        </div>
      </section>

      <section className="work-panel observability-diagnostic-grid observability-workbench">
        <div className="observability-panel observability-panel--feed observability-column">
          <div className="observability-section-header">
            <div className="page-heading">
              <span>{t("web.observability.feed.eyebrow", "Diagnostic feed")}</span>
              <h1>{t("web.observability.feed.title", "统一诊断流")}</h1>
              <p>{t("web.observability.feed.description", "列表按时间统一展示事件；需要收窄范围时，展开高级筛选按类型、严重程度或 ID 精确定位。")}</p>
            </div>
            <div className="observability-feed-count">
              {`${filteredFeed.length} ${t("web.observability.feed.count", "条")}`}
            </div>
          </div>

          <div className="observability-feed-tools">
            <input
              className="observability-search"
              value={filters.q}
              onChange={(event) => setFilters((current) => ({ ...current, q: event.target.value }))}
              placeholder={t("web.observability.feed.search", "搜索消息、事件、trace_id、request_id 或组件")}
            />
            <details className="observability-filter-popover">
              <summary className="secondary">{t("web.observability.filters.title", "高级筛选")}</summary>
              <div className="observability-filter-popover__panel">
                <div className="observability-filter-grid">
                  <label className="observability-filter-field">
                    <span>{t("web.observability.filters.scope", "范围")}</span>
                    <MultiSelect
                      values={filters.scopes}
                      onChange={(values) => setFilters((current) => ({ ...current, scopes: values as UnifiedFilters["scopes"] }))}
                      options={scopeFilterOptions}
                      placeholder={t("web.observability.filters.scope_all", "全部范围")}
                    />
                  </label>
                  <label className="observability-filter-field">
                    <span>{t("web.observability.filters.signal_type", "类型")}</span>
                    <MultiSelect
                      values={filters.signalTypes}
                      onChange={(values) => setFilters((current) => ({ ...current, signalTypes: values as UnifiedFilters["signalTypes"] }))}
                      options={signalTypeFilterOptions}
                      placeholder={t("web.observability.filters.signal_type_all", "全部类型")}
                    />
                  </label>
                  <label className="observability-filter-field">
                    <span>{t("web.observability.filters.component", "组件")}</span>
                    <MultiSelect
                      values={filters.components}
                      onChange={(values) => setFilters((current) => ({ ...current, components: values }))}
                      options={componentFilterOptions}
                      placeholder={t("web.observability.filters.all", "全部")}
                    />
                  </label>
                  <label className="observability-filter-field">
                    <span>{t("web.observability.filters.source_kind", "来源")}</span>
                    <MultiSelect
                      values={filters.sourceKinds}
                      onChange={(values) => setFilters((current) => ({ ...current, sourceKinds: values }))}
                      options={sourceKindFilterOptions}
                      placeholder={t("web.observability.filters.all", "全部")}
                    />
                  </label>
                  <label className="observability-filter-field">
                    <span>{t("web.observability.filters.log_level", "日志等级")}</span>
                    <MultiSelect
                      values={filters.logLevels}
                      onChange={(values) => setFilters((current) => ({ ...current, logLevels: values }))}
                      options={logLevelFilterOptions}
                      placeholder={t("web.observability.filters.all", "全部")}
                    />
                  </label>
                  <label className="observability-filter-field">
                    <span>{t("web.observability.filters.trace_status", "链路结果")}</span>
                    <MultiSelect
                      values={filters.traceStatuses}
                      onChange={(values) => setFilters((current) => ({ ...current, traceStatuses: values }))}
                      options={traceStatusFilterOptions}
                      placeholder={t("web.observability.filters.all", "全部")}
                    />
                  </label>
                  <label className="observability-filter-field">
                    <span>{t("web.observability.filters.request_id", "请求编号")}</span>
                    <input
                      value={filters.requestId}
                      onChange={(event) => setFilters((current) => ({ ...current, requestId: event.target.value }))}
                      placeholder={t("web.observability.filters.request_id_placeholder", "输入 request_id")}
                    />
                  </label>
                  <label className="observability-filter-field">
                    <span>{t("web.observability.filters.trace_id", "链路编号")}</span>
                    <input
                      value={filters.traceId}
                      onChange={(event) => setFilters((current) => ({ ...current, traceId: event.target.value }))}
                      placeholder={t("web.observability.filters.trace_id_placeholder", "输入 trace_id")}
                    />
                  </label>
                </div>
              </div>
            </details>
          </div>

          <div className="observability-scroll stack">
            {filteredFeed.length === 0 ? (
              <div className="empty-card">{t("web.observability.feed.empty", "当前筛选下没有可显示的诊断事件。")}</div>
            ) : (
              filteredFeed.map((item) => (
                <article
                  key={item.key}
                  className={`resource-card observability-feed-card observability-feed-card--${item.priority} ${selectedItemKey === item.key ? "observability-item--active" : ""}`}
                >
                  <button type="button" className="plain-card-button" onClick={() => setSelectedItemKey(item.key)}>
                    <header className="observability-feed-card__header">
                      <div className="stack">
                        <strong>{item.title}</strong>
                        <span className="observability-feed-card__kind">
                          {localizeSignalType(item.kind, t)}
                        </span>
                      </div>
                      <span className="observability-meta">
                        <span className={`badge ${item.badgeClass}`}>
                          {item.kind === "log"
                            ? localizeLogLevel(item.badgeValue, t)
                            : localizeTraceStatus(item.badgeValue, t)}
                        </span>
                        <span className="badge badge--muted">{item.component}</span>
                      </span>
                    </header>
                    <p>{item.summary}</p>
                    <div className="observability-inline-meta">
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

        <div className="observability-panel observability-panel--detail observability-column">
          <div className="observability-section-header">
            <div className="page-heading">
              <span>{t("web.observability.detail.eyebrow", "Context")}</span>
              <h1>{t("web.observability.detail.title", "关联上下文")}</h1>
              <p>{t("web.observability.detail.description", "这里统一展示选中事件的关键信息、关联日志和 trace 链路。")}</p>
            </div>
            {loadingTraceDetail ? <span>{t("web.common.loading", "加载中…")}</span> : null}
          </div>

          <div className="details-panel observability-detail">
            {selectedItem ? (
              <div className="stack">
                <section className="observability-detail-block">
                  <div className="observability-detail-block__header">
                    <strong>{t("web.observability.detail.summary", "概览")}</strong>
                  </div>
                  <div className="kv-list observability-kv-list">
                    <span>{t("web.observability.detail.signal", "类型")}</span>
                    <strong>{localizeSignalType(selectedItem.kind, t)}</strong>
                    <span>{t("web.observability.detail.time", "时间")}</span>
                    <strong>{formatDateTime(selectedItem.timestamp)}</strong>
                    <span>{t("web.observability.detail.component", "组件")}</span>
                    <strong>{selectedItem.component}</strong>
                    <span>{t("web.observability.detail.source", "来源")}</span>
                    <strong>{localizeSourceKind(selectedItem.sourceKind, t)}</strong>
                    <span>{t("web.observability.filters.request_id", "请求编号")}</span>
                    <strong>{selectedItem.requestId ?? t("web.common.none", "无")}</strong>
                    <span>{t("web.observability.filters.trace_id", "链路编号")}</span>
                    <strong>{selectedItem.traceId ?? t("web.common.none", "无")}</strong>
                  </div>
                </section>

                {selectedItem.kind === "log" && selectedItem.log ? (
                  <section className="observability-detail-block">
                    <div className="observability-detail-block__header">
                      <strong>{t("web.observability.detail.core", "核心信息")}</strong>
                    </div>
                    <div className="mini-card observability-context-card">
                      <strong>{selectedItem.log.event}</strong>
                      <p>{selectedItem.log.message}</p>
                      <div className="observability-meta">
                        <span className={`badge ${levelBadgeClass(selectedItem.log.level)}`}>{localizeLogLevel(selectedItem.log.level, t)}</span>
                        {selectedItem.log.span_id ? <span className="badge badge--muted">span:{selectedItem.log.span_id}</span> : null}
                      </div>
                    </div>
                  </section>
                ) : null}

                {selectedItem.kind === "trace" && selectedItem.trace ? (
                  <section className="observability-detail-block">
                    <div className="observability-detail-block__header">
                      <strong>{t("web.observability.detail.core", "核心信息")}</strong>
                    </div>
                    <div className="mini-card observability-context-card">
                      <strong>{selectedItem.trace.name}</strong>
                      <p>{`${selectedItem.trace.spanCount} ${t("web.observability.metrics.spans_suffix", "spans")} · ${selectedItem.trace.durationMs} ms`}</p>
                      <div className="observability-meta">
                        <span className={`badge ${statusBadgeClass(selectedItem.trace.status)}`}>{localizeTraceStatus(selectedItem.trace.status, t)}</span>
                        <span className="badge badge--muted">{selectedItem.trace.kind}</span>
                        {selectedItem.trace.sourceId ? <span className="badge badge--muted">{selectedItem.trace.sourceId}</span> : null}
                      </div>
                    </div>
                    {selectedTraceSummary ? (
                      <div className="kv-list observability-kv-list">
                        <span>{t("web.observability.detail.duration", "耗时")}</span>
                        <strong>{`${selectedTraceSummary.durationMs} ms`}</strong>
                        <span>{t("web.observability.detail.spans", "Span 数")}</span>
                        <strong>{selectedTraceSummary.spanCount}</strong>
                        <span>{t("web.observability.detail.started_at", "开始时间")}</span>
                        <strong>{formatDateTime(selectedTraceSummary.startedAt)}</strong>
                        <span>{t("web.observability.detail.ended_at", "结束时间")}</span>
                        <strong>{formatDateTime(selectedTraceSummary.endedAt)}</strong>
                      </div>
                    ) : null}
                  </section>
                ) : null}

                {selectedTraceSummary ? (
                  <section className="observability-detail-block">
                    <div className="observability-detail-block__header">
                      <strong>{t("web.observability.detail.related_trace", "关联链路")}</strong>
                    </div>
                    <div className="mini-card observability-context-card">
                      <strong>{selectedTraceSummary.name}</strong>
                      <div className="observability-inline-meta">
                        <span>{selectedTraceSummary.traceId}</span>
                        <span>{`${selectedTraceSummary.spanCount} ${t("web.observability.metrics.spans_suffix", "spans")}`}</span>
                        <span>{`${selectedTraceSummary.durationMs} ms`}</span>
                      </div>
                    </div>

                    {previewTraceSpans.length > 0 ? (
                      <div className="timeline-list">
                        {previewTraceSpans.map((span) => (
                          <article key={span.id} className="timeline-item observability-timeline-item">
                            <div className="stack">
                              <strong>{span.duration_ms} ms</strong>
                              <small>{formatDateTime(span.started_at)}</small>
                            </div>
                            <div className="stack">
                              <div className="observability-section-header">
                                <strong>{span.name}</strong>
                                <span className="observability-meta">
                                  <span className={`badge ${statusBadgeClass(span.status)}`}>{localizeTraceStatus(span.status, t)}</span>
                                  <span className="badge badge--muted">{span.kind}</span>
                                </span>
                              </div>
                              <div className="observability-meta">
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
                      <details className="details-panel observability-detail-disclosure">
                        <summary>{t("web.observability.detail.expand_timeline", "展开完整链路")}</summary>
                        <div className="timeline-list">
                          {sortedTraceSpans.map((span) => (
                            <article key={span.id} className="timeline-item observability-timeline-item">
                              <div className="stack">
                                <strong>{span.duration_ms} ms</strong>
                                <small>{formatDateTime(span.started_at)}</small>
                              </div>
                              <div className="stack">
                                <div className="observability-section-header">
                                  <strong>{span.name}</strong>
                                  <span className="observability-meta">
                                    <span className={`badge ${statusBadgeClass(span.status)}`}>{localizeTraceStatus(span.status, t)}</span>
                                    <span className="badge badge--muted">{span.kind}</span>
                                  </span>
                                </div>
                                <div className="observability-meta">
                                  <span className="badge badge--muted">{span.component}</span>
                                  <span className="badge badge--muted">{localizeSourceKind(span.source_kind, t)}</span>
                                  {span.source_id ? <span className="badge badge--muted">{span.source_id}</span> : null}
                                  {span.parent_span_id ? <span className="badge badge--muted">parent:{span.parent_span_id}</span> : null}
                                </div>
                                <pre className="observability-json">{stringifyJson(span.attributes)}</pre>
                              </div>
                            </article>
                          ))}
                        </div>
                      </details>
                    ) : null}
                  </section>
                ) : null}

                <section className="observability-detail-block">
                  <div className="observability-detail-block__header">
                    <strong>{t("web.observability.detail.related_logs", "关联日志")}</strong>
                  </div>
                  <div className="observability-related-logs">
                    {selectedItem.kind === "log" && selectedItem.log ? (
                      <article className="log-card observability-related-log observability-related-log--current">
                        <header className="observability-related-log__header">
                          <div className="stack observability-related-log__title">
                            <small>{t("web.observability.detail.current_log", "当前日志")}</small>
                            <strong>{selectedItem.log.event}</strong>
                          </div>
                          <span className="observability-meta">
                            <span className={`badge ${levelBadgeClass(selectedItem.log.level)}`}>{localizeLogLevel(selectedItem.log.level, t)}</span>
                            <span className="badge badge--muted">{selectedItem.log.component}</span>
                          </span>
                        </header>
                        <p className="observability-related-log__message">{selectedItem.log.message}</p>
                        <div className="observability-inline-meta observability-related-log__meta">
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
                        <article key={item.id} className="log-card observability-related-log">
                          <header className="observability-related-log__header">
                            <div className="stack observability-related-log__title">
                              <strong>{item.event}</strong>
                              <small>{formatDateTime(item.created_at)}</small>
                            </div>
                            <span className="observability-meta">
                              <span className={`badge ${levelBadgeClass(item.level)}`}>{localizeLogLevel(item.level, t)}</span>
                              <span className="badge badge--muted">{item.component}</span>
                            </span>
                          </header>
                          <p className="observability-related-log__message">{item.message}</p>
                          <div className="observability-inline-meta observability-related-log__meta">
                            <span>{localizeSourceKind(item.source_kind, t)}</span>
                            {item.request_id ? <span>request:{item.request_id}</span> : null}
                            {item.trace_id ? <span>trace:{item.trace_id}</span> : null}
                            {item.span_id ? <span>span:{item.span_id}</span> : null}
                          </div>
                        </article>
                      ))
                    ) : (
                      <div className="empty-card observability-related-log-empty">
                        <strong>{t("web.observability.detail.related_logs_empty_title", "没有更多关联日志")}</strong>
                        <p>{t("web.observability.detail.related_logs_empty", "当前上下文里没有更多关联日志。")}</p>
                      </div>
                    )}
                  </div>
                </section>

                {selectedItem.kind === "log" && selectedItem.log ? (
                  <section className="observability-detail-block">
                    <div className="observability-detail-block__header">
                      <strong>{t("web.observability.common.attributes", "属性")}</strong>
                    </div>
                    <pre className="observability-json">{stringifyJson(selectedItem.log.attributes)}</pre>
                  </section>
                ) : null}
              </div>
            ) : (
              <div className="empty-card">{t("web.observability.detail.empty", "选择一条事件后在这里查看关联日志和链路。")}</div>
            )}
          </div>
        </div>
      </section>
    </div>
  );
}
