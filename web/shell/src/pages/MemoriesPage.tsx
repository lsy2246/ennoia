import { useEffect, useState, type FormEvent } from "react";

import {
  createMemory,
  listMemories,
  recallMemories,
  reviewMemory,
  type Memory,
} from "../api";
import { useAuthStore } from "../stores/auth";

export function MemoriesPage() {
  const user = useAuthStore((s) => s.user);
  const [memories, setMemories] = useState<Memory[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [ownerKind, setOwnerKind] = useState("agent");
  const [ownerId, setOwnerId] = useState("coder");
  const [namespace, setNamespace] = useState("user/profile");
  const [kind, setKind] = useState("fact");
  const [stability, setStability] = useState<"working" | "long_term">("working");
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [sourceRef, setSourceRef] = useState("");

  const [recallQuery, setRecallQuery] = useState("");
  const [recallMode, setRecallMode] = useState<"namespace" | "fts">("fts");

  async function refresh() {
    try {
      setMemories(await listMemories());
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    try {
      const sources =
        stability === "long_term" && sourceRef
          ? [{ kind: "user_directive", reference: sourceRef }]
          : undefined;
      await createMemory({
        owner_kind: ownerKind,
        owner_id: ownerId,
        namespace,
        memory_kind: kind,
        stability,
        title: title || undefined,
        content,
        sources,
      });
      setTitle("");
      setContent("");
      setSourceRef("");
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleRecall(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      const result = await recallMemories({
        owner_kind: ownerKind,
        owner_id: ownerId,
        query_text: recallQuery || undefined,
        mode: recallMode,
        limit: 50,
      });
      setMemories(result.memories);
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleReview(
    memoryId: string,
    action: "approve" | "reject" | "supersede" | "retire",
  ) {
    try {
      await reviewMemory({
        target_memory_id: memoryId,
        reviewer: user?.username ?? "unknown",
        action,
      });
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="page">
      <h1>Memories</h1>
      {error && <div className="error">{error}</div>}

      <section className="settings-grid">
        <form onSubmit={handleCreate} className="form-stack">
          <h3>Remember</h3>
          <div className="form-row">
            <label>
              Owner kind
              <select value={ownerKind} onChange={(e) => setOwnerKind(e.target.value)}>
                <option value="agent">agent</option>
                <option value="space">space</option>
                <option value="global">global</option>
              </select>
            </label>
            <label>
              Owner ID
              <input value={ownerId} onChange={(e) => setOwnerId(e.target.value)} />
            </label>
          </div>
          <label>
            Namespace
            <input value={namespace} onChange={(e) => setNamespace(e.target.value)} />
          </label>
          <div className="form-row">
            <label>
              Kind
              <select value={kind} onChange={(e) => setKind(e.target.value)}>
                <option value="fact">fact</option>
                <option value="preference">preference</option>
                <option value="decision_note">decision_note</option>
                <option value="procedure">procedure</option>
                <option value="context">context</option>
                <option value="observation">observation</option>
              </select>
            </label>
            <label>
              Stability
              <select
                value={stability}
                onChange={(e) => setStability(e.target.value as never)}
              >
                <option value="working">working</option>
                <option value="long_term">long_term (requires source)</option>
              </select>
            </label>
          </div>
          <label>
            Title (optional)
            <input value={title} onChange={(e) => setTitle(e.target.value)} />
          </label>
          <label>
            Content
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              rows={4}
              required
            />
          </label>
          {stability === "long_term" && (
            <label>
              Source reference
              <input
                value={sourceRef}
                onChange={(e) => setSourceRef(e.target.value)}
                placeholder="e.g. https://... or doc:..."
                required
              />
            </label>
          )}
          <button type="submit">Save memory</button>
        </form>

        <form onSubmit={handleRecall} className="form-stack">
          <h3>Recall</h3>
          <label>
            Query
            <input
              value={recallQuery}
              onChange={(e) => setRecallQuery(e.target.value)}
              placeholder="keywords (FTS) or blank"
            />
          </label>
          <label>
            Mode
            <select value={recallMode} onChange={(e) => setRecallMode(e.target.value as never)}>
              <option value="fts">fts</option>
              <option value="namespace">namespace</option>
            </select>
          </label>
          <button type="submit">Search</button>
        </form>
      </section>

      <section>
        <h3>Results ({memories.length})</h3>
        <table className="table">
          <thead>
            <tr>
              <th>Namespace</th>
              <th>Kind</th>
              <th>Title / content</th>
              <th>Status</th>
              <th>Stability</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {memories.map((m) => (
              <tr key={m.id}>
                <td><code>{m.namespace}</code></td>
                <td>{m.memory_kind}</td>
                <td>
                  <strong>{m.title ?? "(no title)"}</strong>
                  <br />
                  <small>{m.content}</small>
                </td>
                <td><span className={`pill pill--${m.status}`}>{m.status}</span></td>
                <td>{m.stability}</td>
                <td className="row-actions">
                  <button onClick={() => handleReview(m.id, "approve")}>Approve</button>
                  <button onClick={() => handleReview(m.id, "retire")} className="danger">
                    Retire
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
