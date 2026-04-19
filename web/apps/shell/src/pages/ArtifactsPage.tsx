import { useEffect, useState } from "react";

import { loadWorkspaceSnapshot, type WorkspaceSnapshot } from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function ArtifactsPage() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
  const { formatDateTime } = useUiHelpers();

  useEffect(() => {
    loadWorkspaceSnapshot().then(setSnapshot).catch(() => setSnapshot(null));
  }, []);

  if (!snapshot) {
    return <div className="page"><p>Loading artifacts…</p></div>;
  }

  return (
    <div className="page">
      <h1>Artifacts</h1>
      <table className="table">
        <thead>
          <tr>
            <th>ID</th>
            <th>Run</th>
            <th>Conversation</th>
            <th>Type</th>
            <th>Path</th>
            <th>Created</th>
          </tr>
        </thead>
        <tbody>
          {snapshot.artifacts.map((artifact) => (
            <tr key={artifact.id}>
              <td><code>{artifact.id.slice(0, 12)}</code></td>
              <td><code>{artifact.run_id.slice(0, 12)}</code></td>
              <td>{artifact.conversation_id ?? "—"}</td>
              <td>{artifact.kind}</td>
              <td><code>{artifact.relative_path}</code></td>
              <td>{formatDateTime(artifact.created_at)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
