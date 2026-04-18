import { useEffect, useState, type FormEvent } from "react";

import {
  adminCreateApiKey,
  adminDeleteApiKey,
  adminListApiKeys,
  adminListUsers,
  type ApiKey,
  type User,
} from "../api";

export function AdminApiKeysPage() {
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [users, setUsers] = useState<User[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [userId, setUserId] = useState("");
  const [label, setLabel] = useState("");
  const [scopes, setScopes] = useState("");
  const [revealedKey, setRevealedKey] = useState<string | null>(null);

  async function refresh() {
    try {
      const [k, u] = await Promise.all([adminListApiKeys(), adminListUsers()]);
      setKeys(k);
      setUsers(u);
      if (!userId && u.length > 0) setUserId(u[0].id);
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
    setRevealedKey(null);
    try {
      const result = await adminCreateApiKey({
        user_id: userId,
        label: label || undefined,
        scopes: scopes.split(",").map((s) => s.trim()).filter(Boolean),
      });
      setRevealedKey(result.raw_key);
      setLabel("");
      setScopes("");
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleDelete(id: string) {
    if (!confirm("Revoke this API key? It cannot be undone.")) return;
    await adminDeleteApiKey(id);
    await refresh();
  }

  return (
    <div className="page">
      <h1>API Keys</h1>
      {error && <div className="error">{error}</div>}

      <section>
        <h3>Create key</h3>
        <form onSubmit={handleCreate} className="inline-form">
          <select value={userId} onChange={(e) => setUserId(e.target.value)} required>
            {users.map((u) => (
              <option key={u.id} value={u.id}>
                {u.username}
              </option>
            ))}
          </select>
          <input
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="label (optional)"
          />
          <input
            value={scopes}
            onChange={(e) => setScopes(e.target.value)}
            placeholder="scopes, comma separated"
          />
          <button type="submit">Generate</button>
        </form>
        {revealedKey && (
          <div className="reveal-box">
            <strong>New API key (copy now, it won't be shown again):</strong>
            <pre>{revealedKey}</pre>
          </div>
        )}
      </section>

      <table className="table">
        <thead>
          <tr>
            <th>ID</th>
            <th>User</th>
            <th>Label</th>
            <th>Scopes</th>
            <th>Created</th>
            <th>Last used</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {keys.map((k) => (
            <tr key={k.id}>
              <td><code>{k.id.slice(0, 10)}</code></td>
              <td><code>{k.user_id.slice(0, 10)}</code></td>
              <td>{k.label ?? "—"}</td>
              <td>{k.scopes.join(", ") || "—"}</td>
              <td>{new Date(k.created_at).toLocaleDateString()}</td>
              <td>{k.last_used_at ? new Date(k.last_used_at).toLocaleString() : "—"}</td>
              <td>
                <button className="danger" onClick={() => handleDelete(k.id)}>
                  Revoke
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
