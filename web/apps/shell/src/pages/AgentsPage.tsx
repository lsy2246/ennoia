import { useEffect, useState } from "react";

import { loadWorkspaceSnapshot, type WorkspaceSnapshot } from "@ennoia/api-client";

export function AgentsPage() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);

  useEffect(() => {
    loadWorkspaceSnapshot().then(setSnapshot).catch(() => setSnapshot(null));
  }, []);

  if (!snapshot) {
    return <div className="page"><p>Loading agents…</p></div>;
  }

  return (
    <div className="page">
      <h1>Agents</h1>
      <table className="table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Model</th>
            <th>Workspace</th>
            <th>Artifacts</th>
          </tr>
        </thead>
        <tbody>
          {snapshot.agents.map((agent) => (
            <tr key={agent.id}>
              <td>{agent.display_name} ({agent.id})</td>
              <td>{agent.default_model}</td>
              <td><code>{agent.workspace_dir}</code></td>
              <td><code>{agent.artifacts_dir}</code></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
