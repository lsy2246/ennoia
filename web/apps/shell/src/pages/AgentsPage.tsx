import { PageHeader } from "@/components/PageHeader";
import { useWorkspaceSnapshot } from "@/hooks/useWorkspaceSnapshot";
import { useUiHelpers } from "@/stores/ui";

export function AgentsPage() {
  const { snapshot, loading, error, refresh } = useWorkspaceSnapshot();
  const { t } = useUiHelpers();

  if (loading || !snapshot) {
    return <div className="page"><p>{t("shell.loading.agents", "Loading agents…")}</p></div>;
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.agents.title", "Agents")}
        description={t(
          "shell.page.agents.description",
          "Inspect agent configuration, default model, skill directories and runtime workspaces.",
        )}
        meta={[`${snapshot.agents.length} ${t("shell.meta.total", "total")}`]}
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
              <th>{t("shell.agents.name", "Agent")}</th>
              <th>{t("shell.agents.kind", "Kind")}</th>
              <th>{t("shell.agents.model", "Model")}</th>
              <th>{t("shell.agents.workspace_mode", "Workspace mode")}</th>
              <th>{t("shell.extensions.skills", "Skills dir")}</th>
              <th>{t("shell.agents.workspace", "Workspace")}</th>
              <th>{t("shell.agents.artifacts", "Artifacts")}</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.agents.map((agent) => (
              <tr key={agent.id}>
                <td>
                  <strong>{agent.display_name}</strong>
                  <div className="muted">{agent.id}</div>
                </td>
                <td>{agent.kind}</td>
                <td>{agent.default_model}</td>
                <td>{agent.workspace_mode}</td>
                <td><code>{agent.skills_dir}</code></td>
                <td><code>{agent.workspace_dir}</code></td>
                <td><code>{agent.artifacts_dir}</code></td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
