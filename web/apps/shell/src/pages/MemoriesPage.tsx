import { useEffect, useMemo, useState } from "react";

import {
  createMemory,
  recallMemories,
  reviewMemory,
  type RecallResult,
} from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useWorkspaceSnapshot } from "@/hooks/useWorkspaceSnapshot";
import { useUiHelpers } from "@/stores/ui";

export function MemoriesPage() {
  const { snapshot, loading, error, refresh } = useWorkspaceSnapshot();
  const { t, formatDateTime } = useUiHelpers();
  const [ownerKind, setOwnerKind] = useState<"global" | "space" | "agent">("global");
  const [ownerId, setOwnerId] = useState("workspace");
  const [namespace, setNamespace] = useState("workspace/default");
  const [memoryKind, setMemoryKind] = useState("fact");
  const [stability, setStability] = useState("working");
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [queryText, setQueryText] = useState("");
  const [mode, setMode] = useState<"namespace" | "fts" | "hybrid">("hybrid");
  const [recall, setRecall] = useState<RecallResult | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const ownerOptions = useMemo(() => {
    if (!snapshot) {
      return [{ value: "workspace", label: "workspace" }];
    }
    if (ownerKind === "space") {
      return snapshot.spaces.map((space) => ({
        value: space.id,
        label: `${space.display_name} (${space.id})`,
      }));
    }
    if (ownerKind === "agent") {
      return snapshot.agents.map((agent) => ({
        value: agent.id,
        label: `${agent.display_name} (${agent.id})`,
      }));
    }
    return [{ value: "workspace", label: "workspace" }];
  }, [ownerKind, snapshot]);

  useEffect(() => {
    setOwnerId(ownerOptions[0]?.value ?? "workspace");
  }, [ownerOptions]);

  async function handleCreateMemory() {
    setBusy(true);
    setSubmitError(null);
    setMessage(null);
    try {
      await createMemory({
        owner_kind: ownerKind,
        owner_id: ownerId,
        namespace,
        memory_kind: memoryKind,
        stability,
        title: title || undefined,
        content,
      });
      setTitle("");
      setContent("");
      setMessage(t("shell.memories.created", "Memory stored."));
      await refresh();
    } catch (err) {
      setSubmitError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleRecall() {
    setBusy(true);
    setSubmitError(null);
    try {
      const result = await recallMemories({
        owner_kind: ownerKind,
        owner_id: ownerId,
        namespace_prefix: namespace || undefined,
        query_text: queryText || undefined,
        mode,
      });
      setRecall(result);
    } catch (err) {
      setSubmitError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleReview(memoryId: string, action: "approve" | "reject" | "retire") {
    setSubmitError(null);
    setMessage(null);
    try {
      await reviewMemory({
        target_memory_id: memoryId,
        reviewer: "shell",
        action,
      });
      setMessage(t("shell.memories.reviewed", "Memory review applied."));
      await refresh();
    } catch (err) {
      setSubmitError(String(err));
    }
  }

  if (loading || !snapshot) {
    return <div className="page"><p>{t("shell.loading.memories", "Loading memories…")}</p></div>;
  }

  const activeMemories = snapshot.memories.filter((memory) => memory.status === "active").length;

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.memories.title", "Memories")}
        description={t(
          "shell.page.memories.description",
          "Inspect stored memories, recall context and review records from the control plane.",
        )}
        meta={[
          `${snapshot.memories.length} ${t("shell.meta.total", "total")}`,
          `${activeMemories} ${t("shell.memories.active", "active")}`,
        ]}
        actions={
          <button className="secondary" onClick={() => void refresh()}>
            {t("shell.action.refresh", "Refresh")}
          </button>
        }
      />

      {error && <div className="error">{error}</div>}
      {submitError && <div className="error">{submitError}</div>}
      {message && <div className="success">{message}</div>}

      <section className="surface-card">
        <h2>{t("shell.memories.create", "Create memory")}</h2>
        <div className="form-row">
          <label>
            {t("shell.memories.owner_kind", "Owner kind")}
            <select
              value={ownerKind}
              onChange={(event) => setOwnerKind(event.target.value as "global" | "space" | "agent")}
            >
              <option value="global">{t("shell.owner.global", "Global")}</option>
              <option value="space">{t("shell.owner.space", "Space")}</option>
              <option value="agent">{t("shell.owner.agent", "Agent")}</option>
            </select>
          </label>
          <label>
            {t("shell.memories.owner", "Owner")}
            <select value={ownerId} onChange={(event) => setOwnerId(event.target.value)}>
              {ownerOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Namespace
            <input value={namespace} onChange={(event) => setNamespace(event.target.value)} />
          </label>
        </div>
        <div className="form-row">
          <label>
            {t("shell.memories.kind", "Memory kind")}
            <select value={memoryKind} onChange={(event) => setMemoryKind(event.target.value)}>
              <option value="fact">fact</option>
              <option value="preference">preference</option>
              <option value="decision_note">decision_note</option>
              <option value="procedure">procedure</option>
              <option value="context">context</option>
              <option value="observation">observation</option>
            </select>
          </label>
          <label>
            {t("shell.memories.stability", "Stability")}
            <select value={stability} onChange={(event) => setStability(event.target.value)}>
              <option value="working">working</option>
              <option value="long_term">long_term</option>
            </select>
          </label>
          <label>
            {t("shell.memories.title_field", "Title")}
            <input value={title} onChange={(event) => setTitle(event.target.value)} />
          </label>
        </div>
        <label>
          {t("shell.memories.content", "Content")}
          <textarea value={content} onChange={(event) => setContent(event.target.value)} rows={4} />
        </label>
        <div className="actions">
          <button onClick={handleCreateMemory} disabled={busy || content.trim().length === 0}>
            {busy ? t("shell.action.saving", "Saving…") : t("shell.memories.create_action", "Store memory")}
          </button>
        </div>
      </section>

      <section className="surface-card">
        <h2>{t("shell.memories.recall", "Recall memories")}</h2>
        <div className="form-row">
          <label>
            {t("shell.memories.query", "Query")}
            <input value={queryText} onChange={(event) => setQueryText(event.target.value)} />
          </label>
          <label>
            Mode
            <select value={mode} onChange={(event) => setMode(event.target.value as "namespace" | "fts" | "hybrid")}>
              <option value="namespace">namespace</option>
              <option value="fts">fts</option>
              <option value="hybrid">hybrid</option>
            </select>
          </label>
        </div>
        <div className="actions">
          <button className="secondary" onClick={handleRecall} disabled={busy}>
            {busy ? t("shell.action.loading", "Loading…") : t("shell.memories.recall_action", "Recall")}
          </button>
        </div>
        {recall ? (
          <div className="surface-card surface-card--subtle">
            <p>
              <strong>{recall.memories.length}</strong> {t("shell.memories.recall_results", "results")} ·
              <span> {recall.mode}</span>
            </p>
            <ul className="simple-list">
              {recall.memories.map((memory) => (
                <li key={memory.id}>
                  <strong>{memory.title ?? memory.namespace}</strong>
                  <span>{memory.summary ?? memory.content}</span>
                </li>
              ))}
            </ul>
          </div>
        ) : null}
      </section>

      <section>
        <h2>{t("shell.memories.list", "Stored memories")}</h2>
        <table className="table">
          <thead>
            <tr>
              <th>{t("shell.memories.item", "Memory")}</th>
              <th>Namespace</th>
              <th>{t("shell.memories.kind", "Kind")}</th>
              <th>{t("shell.memories.status", "Status")}</th>
              <th>{t("shell.memories.updated_at", "Updated")}</th>
              <th>{t("shell.memories.actions", "Actions")}</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.memories.map((memory) => (
              <tr key={memory.id}>
                <td>
                  <strong>{memory.title ?? memory.id.slice(0, 12)}</strong>
                  <div className="muted">{memory.summary ?? memory.content.slice(0, 80)}</div>
                </td>
                <td><code>{memory.namespace}</code></td>
                <td>{memory.memory_kind}</td>
                <td><span className={`pill pill--${memory.status}`}>{memory.status}</span></td>
                <td>{formatDateTime(memory.updated_at)}</td>
                <td>
                  <div className="row-actions">
                    <button className="secondary" onClick={() => void handleReview(memory.id, "approve")}>
                      approve
                    </button>
                    <button className="secondary" onClick={() => void handleReview(memory.id, "retire")}>
                      retire
                    </button>
                    <button className="danger" onClick={() => void handleReview(memory.id, "reject")}>
                      reject
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
