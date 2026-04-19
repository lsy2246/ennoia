import { useEffect, useState } from "react";

import { loadWorkspaceSnapshot, type WorkspaceSnapshot } from "@ennoia/api-client";

export function SpacesPage() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);

  useEffect(() => {
    loadWorkspaceSnapshot().then(setSnapshot).catch(() => setSnapshot(null));
  }, []);

  if (!snapshot) {
    return <div className="page"><p>Loading spaces…</p></div>;
  }

  return (
    <div className="page">
      <h1>Spaces</h1>
      <table className="table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Description</th>
            <th>Primary goal</th>
            <th>Default agents</th>
          </tr>
        </thead>
        <tbody>
          {snapshot.spaces.map((space) => (
            <tr key={space.id}>
              <td>{space.display_name}</td>
              <td>{space.description}</td>
              <td>{space.primary_goal}</td>
              <td>{space.default_agents.join(", ")}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
