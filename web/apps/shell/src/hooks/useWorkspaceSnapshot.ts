import { useCallback, useEffect, useState } from "react";

import { loadWorkspaceSnapshot, type WorkspaceSnapshot } from "@ennoia/api-client";

export function useWorkspaceSnapshot() {
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const nextSnapshot = await loadWorkspaceSnapshot();
      setSnapshot(nextSnapshot);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return {
    snapshot,
    loading,
    error,
    refresh,
  };
}
