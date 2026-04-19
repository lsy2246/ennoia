import { useEffect, useState } from "react";

import {
  adminDeleteSession,
  adminListSessions,
  type Session,
} from "@ennoia/api-client";

export function AdminSessionsPage() {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setSessions(await adminListSessions());
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function handleRevoke(id: string) {
    if (!confirm("Revoke this session?")) return;
    await adminDeleteSession(id);
    await refresh();
  }

  return (
    <div className="page">
      <h1>Active sessions</h1>
      {error && <div className="error">{error}</div>}
      {sessions.length === 0 && <p className="muted">(no sessions)</p>}
      <table className="table">
        <thead>
          <tr>
            <th>ID</th>
            <th>User</th>
            <th>Created</th>
            <th>Expires</th>
            <th>Last seen</th>
            <th>IP / UA</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {sessions.map((s) => (
            <tr key={s.id}>
              <td><code>{s.id.slice(0, 10)}</code></td>
              <td><code>{s.user_id.slice(0, 10)}</code></td>
              <td>{new Date(s.created_at).toLocaleString()}</td>
              <td>{new Date(s.expires_at).toLocaleString()}</td>
              <td>{s.last_seen_at ? new Date(s.last_seen_at).toLocaleString() : "—"}</td>
              <td>
                <small>
                  {s.ip ?? "—"}
                  <br />
                  {s.user_agent ?? "—"}
                </small>
              </td>
              <td>
                <button className="danger" onClick={() => handleRevoke(s.id)}>
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
