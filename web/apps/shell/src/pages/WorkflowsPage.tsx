import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { loadWorkspaceSnapshot, type WorkspaceSnapshot } from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function WorkflowsPage() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
  const { formatDateTime } = useUiHelpers();

  useEffect(() => {
    loadWorkspaceSnapshot().then(setSnapshot).catch(() => setSnapshot(null));
  }, []);

  if (!snapshot) {
    return <div className="page"><p>Loading workflows…</p></div>;
  }

  return (
    <div className="page">
      <h1>Workflows</h1>
      <table className="table">
        <thead>
          <tr>
            <th>ID</th>
            <th>Conversation</th>
            <th>Lane</th>
            <th>Stage</th>
            <th>Goal</th>
            <th>Created</th>
          </tr>
        </thead>
        <tbody>
          {snapshot.runs.map((run) => (
            <tr key={run.id}>
              <td>
                <Link to="/workflows/$runId" params={{ runId: run.id }}>
                  <code>{run.id.slice(0, 12)}</code>
                </Link>
              </td>
              <td>{run.conversation_id}</td>
              <td>{run.lane_id ?? "—"}</td>
              <td>{run.stage}</td>
              <td>{run.goal}</td>
              <td>{formatDateTime(run.created_at)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
