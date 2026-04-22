import { useEffect, useMemo, useState } from "react";

import { listAgents, listChats } from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";
import {
  listMemoryRecords,
  recallMemoryRecords,
  reviewMemoryRecord,
  type MemoryRecord,
} from "./api";

export function MemoryExtensionPage() {
  const { formatDateTime, t } = useUiHelpers();
  const [memories, setMemories] = useState<MemoryRecord[]>([]);
  const [agents, setAgents] = useState<{ id: string; display_name: string }[]>([]);
  const [sessions, setSessions] = useState<{ id: string; title: string }[]>([]);
  const [ownerId, setOwnerId] = useState("all");
  const [kind, setKind] = useState("");
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [reviewer, setReviewer] = useState("operator");
  const [recallResult, setRecallResult] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    try {
      const [nextMemories, nextAgents, nextSessions] = await Promise.all([
        listMemoryRecords(),
        listAgents(),
        listChats(),
      ]);
      setMemories(nextMemories);
      setAgents(nextAgents.map((item) => ({ id: item.id, display_name: item.display_name })));
      setSessions(nextSessions.map((item) => ({ id: item.id, title: item.title })));
    } catch (err) {
      setError(String(err));
    }
  }

  const filtered = useMemo(
    () =>
      memories.filter((memory) => {
        if (ownerId !== "all" && memory.owner.id !== ownerId) {
          return false;
        }
        if (kind && memory.memory_kind !== kind) {
          return false;
        }
        if (!query.trim()) {
          return true;
        }
        const haystack = [
          memory.title,
          memory.summary,
          memory.content,
          memory.namespace,
          memory.owner.id,
        ]
          .filter(Boolean)
          .join("\n")
          .toLowerCase();
        return haystack.includes(query.trim().toLowerCase());
      }),
    [kind, memories, ownerId, query],
  );

  const ownerBreakdown = useMemo(() => {
    const next = new Map<string, number>();
    for (const memory of filtered) {
      const key = `${memory.owner.kind}:${memory.owner.id}`;
      next.set(key, (next.get(key) ?? 0) + 1);
    }
    return [...next.entries()].sort((left, right) => right[1] - left[1]).slice(0, 6);
  }, [filtered]);

  const namespaceBreakdown = useMemo(() => {
    const next = new Map<string, number>();
    for (const memory of filtered) {
      next.set(memory.namespace, (next.get(memory.namespace) ?? 0) + 1);
    }
    return [...next.entries()].sort((left, right) => right[1] - left[1]).slice(0, 6);
  }, [filtered]);

  const stabilityBreakdown = useMemo(() => {
    const next = new Map<string, number>();
    for (const memory of filtered) {
      next.set(memory.stability, (next.get(memory.stability) ?? 0) + 1);
    }
    return [...next.entries()].sort((left, right) => right[1] - left[1]);
  }, [filtered]);

  async function handleRecall() {
    const target = filtered[0];
    if (!target) {
      return;
    }
    try {
      const result = await recallMemoryRecords({
        owner_kind: target.owner.kind,
        owner_id: target.owner.id,
        query_text: query || undefined,
        memory_kind: kind || undefined,
        limit: 8,
        mode: "hybrid",
      });
      setRecallResult(
        `${result.memories.length} memories · ${result.mode} · ${result.total_chars} chars`,
      );
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleReview(memoryId: string, action: string) {
    try {
      await reviewMemoryRecord({
        target_memory_id: memoryId,
        reviewer,
        action,
      });
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  const ownerOptions = [
    ...agents.map((item) => ({ value: item.id, label: `Agent · ${item.display_name}` })),
    ...sessions.map((item) => ({ value: item.id, label: `Conversation · ${item.title}` })),
  ];

  return (
    <div className="resource-layout resource-layout--single">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.memory.eyebrow", "Memory")}</span>
          <h1>{t("web.memory.title", "记忆系统可视化")}</h1>
          <p>
            {t(
              "web.memory.description",
              "在这里按 owner、kind、稳定性和关键词查看记忆，并直接触发 recall 与 review。",
            )}
          </p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        {recallResult ? <div className="success">{recallResult}</div> : null}
        <div className="memory-visual-grid">
          <article className="memory-lane">
            <span>{t("web.memory.metric.total", "当前结果")}</span>
            <strong>{filtered.length}</strong>
            <small>{t("web.memory.metric.total_help", "按当前筛选命中的记忆条目数。")}</small>
          </article>
          <article className="memory-lane">
            <span>{t("web.memory.metric.owners", "Owner 分布")}</span>
            <strong>{ownerBreakdown.length}</strong>
            <small>{ownerBreakdown[0]?.[0] ?? t("web.memory.metric.none", "暂无")}</small>
          </article>
          <article className="memory-lane">
            <span>{t("web.memory.metric.namespaces", "命名空间")}</span>
            <strong>{namespaceBreakdown.length}</strong>
            <small>{namespaceBreakdown[0]?.[0] ?? t("web.memory.metric.none", "暂无")}</small>
          </article>
        </div>
        <div className="filter-bar">
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("web.memory.search_placeholder", "搜索标题、摘要、正文或命名空间")}
          />
          <select value={ownerId} onChange={(event) => setOwnerId(event.target.value)}>
            <option value="all">{t("web.memory.all_owners", "全部 owner")}</option>
            {ownerOptions.map((item) => (
              <option key={item.value} value={item.value}>
                {item.label}
              </option>
            ))}
          </select>
          <select value={kind} onChange={(event) => setKind(event.target.value)}>
            <option value="">{t("web.memory.all_kinds", "全部类型")}</option>
            <option value="fact">fact</option>
            <option value="preference">preference</option>
            <option value="decision_note">decision_note</option>
            <option value="procedure">procedure</option>
            <option value="context">context</option>
            <option value="observation">observation</option>
          </select>
          <button type="button" onClick={() => void refresh()}>
            {t("web.action.refresh", "刷新")}
          </button>
          <button type="button" className="secondary" onClick={() => void handleRecall()}>
            {t("web.memory.recall", "Recall")}
          </button>
        </div>
        <div className="resource-layout">
          <article className="mini-card">
            <div className="panel-title">{t("web.memory.visual_owners", "Owner 热点")}</div>
            <div className="timeline-list">
              {ownerBreakdown.map(([entry, count]) => (
                <div key={entry} className="timeline-item">
                  <strong>{entry}</strong>
                  <span>{count}</span>
                </div>
              ))}
            </div>
          </article>
          <article className="mini-card">
            <div className="panel-title">{t("web.memory.visual_stability", "稳定性")}</div>
            <div className="timeline-list">
              {stabilityBreakdown.map(([entry, count]) => (
                <div key={entry} className="timeline-item">
                  <strong>{entry}</strong>
                  <span>{count}</span>
                </div>
              ))}
            </div>
          </article>
          <article className="mini-card">
            <div className="panel-title">{t("web.memory.visual_namespaces", "命名空间热点")}</div>
            <div className="timeline-list">
              {namespaceBreakdown.map(([entry, count]) => (
                <div key={entry} className="timeline-item">
                  <strong>{entry}</strong>
                  <span>{count}</span>
                </div>
              ))}
            </div>
          </article>
        </div>
        <div className="stack">
          {filtered.map((memory) => (
            <article key={memory.id} className="resource-card">
              <header>
                <strong>{memory.title || memory.namespace}</strong>
                <span>{memory.status}</span>
              </header>
              <p>{memory.summary || memory.content}</p>
              <div className="tag-row">
                <span>
                  {memory.owner.kind}:{memory.owner.id}
                </span>
                <span>{memory.memory_kind}</span>
                <span>{memory.stability}</span>
                <span>{memory.namespace}</span>
              </div>
              <div className="kv-list">
                <span>{t("web.memory.confidence", "置信度")}</span>
                <strong>{memory.confidence}</strong>
                <span>{t("web.memory.importance", "重要度")}</span>
                <strong>{memory.importance}</strong>
                <span>{t("web.memory.updated_at", "更新时间")}</span>
                <strong>{formatDateTime(memory.updated_at)}</strong>
              </div>
              {memory.sources.length > 0 ? (
                <pre className="log-view">
                  {memory.sources.map((item) => `${item.kind}: ${item.reference}`).join("\n")}
                </pre>
              ) : null}
              <div className="button-row">
                <input
                  value={reviewer}
                  onChange={(event) => setReviewer(event.target.value)}
                  placeholder={t("web.memory.reviewer", "reviewer")}
                />
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void handleReview(memory.id, "approve")}
                >
                  approve
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void handleReview(memory.id, "reject")}
                >
                  reject
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void handleReview(memory.id, "supersede")}
                >
                  supersede
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void handleReview(memory.id, "retire")}
                >
                  retire
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
