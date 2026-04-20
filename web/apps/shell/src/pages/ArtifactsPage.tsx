import { PageHeader } from "@/components/PageHeader";
import { useWorkspaceSnapshot } from "@/hooks/useWorkspaceSnapshot";
import { useUiHelpers } from "@/stores/ui";

export function ArtifactsPage() {
  const { snapshot, loading, error, refresh } = useWorkspaceSnapshot();
  const { t, formatDateTime } = useUiHelpers();

  if (loading || !snapshot) {
    return <div className="page"><p>{t("shell.loading.artifacts", "Loading artifacts…")}</p></div>;
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.artifacts.title", "Artifacts")}
        description={t(
          "shell.page.artifacts.description",
          "Browse generated outputs across conversations, runs and lane-level work.",
        )}
        meta={[`${snapshot.artifacts.length} ${t("shell.meta.total", "total")}`]}
        actions={
          <button className="secondary" onClick={() => void refresh()}>
            {t("shell.action.refresh", "Refresh")}
          </button>
        }
      />

      {error && <div className="error">{error}</div>}

      <section>
        <table className="table">
          <thead>
            <tr>
              <th>ID</th>
              <th>{t("shell.artifacts.run", "Run")}</th>
              <th>{t("shell.artifacts.conversation", "Conversation")}</th>
              <th>{t("shell.artifacts.kind", "Kind")}</th>
              <th>{t("shell.artifacts.path", "Path")}</th>
              <th>{t("shell.artifacts.created_at", "Created")}</th>
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
      </section>
    </div>
  );
}
