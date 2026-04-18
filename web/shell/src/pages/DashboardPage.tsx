import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { loadWorkspaceSnapshot, type WorkspaceSnapshot } from "../api";

export function DashboardPage() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    loadWorkspaceSnapshot()
      .then((data) => {
        if (!cancelled) setSnapshot(data);
      })
      .catch((err) => {
        if (!cancelled) setError(String(err));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    return <div className="page"><p className="error">Failed to load: {error}</p></div>;
  }
  if (!snapshot) {
    return <div className="page"><p>Loading…</p></div>;
  }

  return (
    <div className="page">
      <h1>Dashboard</h1>
      <section className="dashboard-grid">
        {Object.entries(snapshot.overview.counts).map(([key, value]) => (
          <div key={key} className="dashboard-card">
            <span className="dashboard-card__value">{value}</span>
            <span className="dashboard-card__label">{key}</span>
          </div>
        ))}
      </section>

      <section>
        <h2>Recent runs</h2>
        <table className="table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Thread</th>
              <th>Stage</th>
              <th>Goal</th>
              <th>Created</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {snapshot.runs.slice(0, 20).map((run) => (
              <tr key={run.id}>
                <td><code>{run.id.slice(0, 12)}</code></td>
                <td>{run.thread_id}</td>
                <td><span className={`stage stage--${run.stage}`}>{run.stage}</span></td>
                <td>{run.goal}</td>
                <td>{new Date(run.created_at).toLocaleString()}</td>
                <td>
                  <Link to="/runs/$runId" params={{ runId: run.id }}>
                    Details
                  </Link>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section>
        <h2>Agents</h2>
        <ul className="simple-list">
          {snapshot.agents.map((a) => (
            <li key={a.id}>
              <strong>{a.display_name}</strong> <code>{a.id}</code> · model:{" "}
              {a.default_model}
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
