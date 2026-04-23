import { useEffect, useMemo, useState } from "react";

import { useUiHelpers } from "@/stores/ui";
import {
  getMemoryWorkspaceSummary,
  listMemoryRecords,
  recallMemoryRecords,
  reviewMemoryRecord,
  type MemoryRecord,
  type WorkspaceSummary,
} from "./api";

type MemoryTab = "truth" | "context" | "review" | "graph";

export default function MemoryExtensionPage() {
  const { formatDateTime, t } = useUiHelpers();
  const [workspace, setWorkspace] = useState<WorkspaceSummary | null>(null);
  const [memories, setMemories] = useState<MemoryRecord[]>([]);
  const [activeTab, setActiveTab] = useState<MemoryTab>("truth");
  const [reviewer, setReviewer] = useState("operator");
  const [search, setSearch] = useState("");
  const [ownerKind, setOwnerKind] = useState("operator");
  const [ownerId, setOwnerId] = useState("local");
  const [conversationId, setConversationId] = useState("");
  const [busy, setBusy] = useState(false);
  const [recallResult, setRecallResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refreshWorkspace();
  }, []);

  async function refreshWorkspace() {
    setError(null);
    try {
      const [nextWorkspace, nextMemories] = await Promise.all([
        getMemoryWorkspaceSummary(),
        listMemoryRecords(),
      ]);
      setWorkspace(nextWorkspace);
      setMemories(nextMemories);
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleRecall() {
    setBusy(true);
    setError(null);
    try {
      const result = await recallMemoryRecords({
        owner_kind: ownerKind.trim() || "operator",
        owner_id: ownerId.trim() || "local",
        conversation_id: conversationId.trim() || undefined,
        query_text: search.trim() || undefined,
        mode: "hybrid",
        limit: 8,
      });
      setRecallResult(`${result.memories.length} memories · ${result.mode} · ${result.total_chars} chars`);
      setActiveTab("context");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleReview(memoryId: string, action: string) {
    setBusy(true);
    setError(null);
    try {
      await reviewMemoryRecord({
        target_memory_id: memoryId,
        reviewer: reviewer.trim() || "operator",
        action,
      });
      await refreshWorkspace();
      setActiveTab("review");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const filteredMemories = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    return memories.filter((memory) => {
      if (!keyword) {
        return true;
      }
      const haystack = [
        memory.title,
        memory.summary,
        memory.content,
        memory.namespace,
        memory.owner.kind,
        memory.owner.id,
      ]
        .filter(Boolean)
        .join("\n")
        .toLowerCase();
      return haystack.includes(keyword);
    });
  }, [memories, search]);

  const reviewCandidates = useMemo(
    () => filteredMemories.filter((memory) => memory.status !== "active"),
    [filteredMemories],
  );

  const ownerStats = useMemo(() => {
    const counts = new Map<string, number>();
    for (const memory of filteredMemories) {
      const key = `${memory.owner.kind}:${memory.owner.id}`;
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return [...counts.entries()].sort((left, right) => right[1] - left[1]).slice(0, 6);
  }, [filteredMemories]);

  const namespaceStats = useMemo(() => {
    const counts = new Map<string, number>();
    for (const memory of filteredMemories) {
      counts.set(memory.namespace, (counts.get(memory.namespace) ?? 0) + 1);
    }
    return [...counts.entries()].sort((left, right) => right[1] - left[1]).slice(0, 6);
  }, [filteredMemories]);

  return (
    <div className="resource-layout" style={{ gridTemplateColumns: "320px minmax(0,1fr)" }}>
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("ext.memory.eyebrow", "Memory")}</span>
          <h1>{t("ext.memory.title", "记忆")}</h1>
          <p>
            {t(
              "ext.memory.description",
              "记忆页只负责记忆检索、上下文装配、审查和图谱概览；会话的新建、打开和消息发送由系统会话页承载。",
            )}
          </p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="memory-visual-grid">
          <article className="memory-lane">
            <span>Active Truth</span>
            <strong>{workspace?.active_memory_count ?? 0}</strong>
            <small>处于 active 状态的长期记忆</small>
          </article>
          <article className="memory-lane">
            <span>Pending Review</span>
            <strong>{workspace?.pending_review_count ?? 0}</strong>
            <small>待人工复核的记忆条目</small>
          </article>
          <article className="memory-lane">
            <span>Context Events</span>
            <strong>{workspace?.message_count ?? 0}</strong>
            <small>记忆引擎观测到的原始上下文</small>
          </article>
        </div>
        <div className="stack">
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="搜索记忆、命名空间、正文"
          />
          <input
            value={ownerKind}
            onChange={(event) => setOwnerKind(event.target.value)}
            placeholder="owner kind"
          />
          <input
            value={ownerId}
            onChange={(event) => setOwnerId(event.target.value)}
            placeholder="owner id"
          />
          <input
            value={conversationId}
            onChange={(event) => setConversationId(event.target.value)}
            placeholder="可选：conversation id"
          />
          <div className="button-row">
            <button type="button" onClick={() => void handleRecall()} disabled={busy}>
              Recall
            </button>
            <button type="button" className="secondary" onClick={() => void refreshWorkspace()} disabled={busy}>
              刷新
            </button>
          </div>
        </div>
      </section>

      <section className="work-panel">
        <div className="button-row">
          <button type="button" className={activeTab === "truth" ? "" : "secondary"} onClick={() => setActiveTab("truth")}>
            Truth
          </button>
          <button type="button" className={activeTab === "context" ? "" : "secondary"} onClick={() => setActiveTab("context")}>
            Context
          </button>
          <button type="button" className={activeTab === "review" ? "" : "secondary"} onClick={() => setActiveTab("review")}>
            Review
          </button>
          <button type="button" className={activeTab === "graph" ? "" : "secondary"} onClick={() => setActiveTab("graph")}>
            Graph
          </button>
        </div>

        {activeTab === "truth" ? (
          <div className="stack">
            {filteredMemories.length === 0 ? (
              <div className="empty-card">当前没有匹配的记忆。</div>
            ) : (
              filteredMemories.map((memory) => (
                <article key={memory.id} className="resource-card">
                  <header>
                    <strong>{memory.title || memory.namespace}</strong>
                    <span>{memory.status}</span>
                  </header>
                  <p>{memory.summary || memory.content}</p>
                  <div className="tag-row">
                    <span>{memory.owner.kind}:{memory.owner.id}</span>
                    <span>{memory.memory_kind}</span>
                    <span>{memory.stability}</span>
                  </div>
                  <div className="kv-list">
                    <span>命名空间</span>
                    <strong>{memory.namespace}</strong>
                    <span>置信度</span>
                    <strong>{memory.confidence}</strong>
                    <span>更新时间</span>
                    <strong>{formatDateTime(memory.updated_at)}</strong>
                  </div>
                </article>
              ))
            )}
          </div>
        ) : null}

        {activeTab === "context" ? (
          <div className="stack">
            <article className="mini-card">
              <div className="panel-title">Context Assembly</div>
              <div className="kv-list">
                <span>消息/事件</span>
                <strong>{workspace?.message_count ?? 0}</strong>
                <span>Graph nodes</span>
                <strong>{workspace?.graph_nodes_count ?? 0}</strong>
                <span>Recall</span>
                <strong>{recallResult ?? "尚未执行"}</strong>
              </div>
            </article>
            <article className="mini-card">
              <div className="panel-title">边界</div>
              <p>
                系统会话页负责原始会话和消息；记忆扩展只消费这些原始记录，生成 truth、recent context、review
                receipts 和 graph sidecar。
              </p>
            </article>
          </div>
        ) : null}

        {activeTab === "review" ? (
          <div className="stack">
            <input
              value={reviewer}
              onChange={(event) => setReviewer(event.target.value)}
              placeholder="reviewer"
            />
            {reviewCandidates.length === 0 ? (
              <div className="empty-card">当前没有待审记忆。</div>
            ) : (
              reviewCandidates.map((memory) => (
                <article key={memory.id} className="resource-card">
                  <header>
                    <strong>{memory.title || memory.namespace}</strong>
                    <span>{memory.status}</span>
                  </header>
                  <p>{memory.summary || memory.content}</p>
                  <div className="button-row">
                    <button type="button" className="secondary" onClick={() => void handleReview(memory.id, "approve")} disabled={busy}>
                      approve
                    </button>
                    <button type="button" className="secondary" onClick={() => void handleReview(memory.id, "reject")} disabled={busy}>
                      reject
                    </button>
                    <button type="button" className="secondary" onClick={() => void handleReview(memory.id, "supersede")} disabled={busy}>
                      supersede
                    </button>
                  </div>
                </article>
              ))
            )}
          </div>
        ) : null}

        {activeTab === "graph" ? (
          <div className="stack">
            <article className="mini-card">
              <div className="panel-title">Graph Overview</div>
              <div className="kv-list">
                <span>节点</span>
                <strong>{workspace?.graph_nodes_count ?? 0}</strong>
                <span>活跃记忆</span>
                <strong>{workspace?.active_memory_count ?? 0}</strong>
                <span>当前结果</span>
                <strong>{filteredMemories.length}</strong>
              </div>
            </article>
            <article className="mini-card">
              <div className="panel-title">Owner 热点</div>
              <div className="kv-list">
                {ownerStats.length === 0 ? (
                  <>
                    <span>暂无</span>
                    <strong>0</strong>
                  </>
                ) : (
                  ownerStats.map(([owner, count]) => (
                    <span key={owner}>
                      {owner}<strong>{count}</strong>
                    </span>
                  ))
                )}
              </div>
            </article>
            <article className="mini-card">
              <div className="panel-title">Namespace 热点</div>
              <div className="kv-list">
                {namespaceStats.length === 0 ? (
                  <>
                    <span>暂无</span>
                    <strong>0</strong>
                  </>
                ) : (
                  namespaceStats.map(([namespace, count]) => (
                    <span key={namespace}>
                      {namespace}<strong>{count}</strong>
                    </span>
                  ))
                )}
              </div>
            </article>
          </div>
        ) : null}
      </section>
    </div>
  );
}
