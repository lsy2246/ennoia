import { PageHeader } from "@/components/PageHeader";
import { useWorkspaceSnapshot } from "@/hooks/useWorkspaceSnapshot";
import { useUiHelpers } from "@/stores/ui";

export function SpacesPage() {
  const { snapshot, loading, error, refresh } = useWorkspaceSnapshot();
  const { t } = useUiHelpers();

  if (loading || !snapshot) {
    return <div className="page"><p>{t("shell.loading.spaces", "Loading spaces…")}</p></div>;
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.spaces.title", "Spaces")}
        description={t(
          "shell.page.spaces.description",
          "Review long-lived collaboration spaces, their goals and default agent sets.",
        )}
        meta={[`${snapshot.spaces.length} ${t("shell.meta.total", "total")}`]}
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
              <th>{t("shell.spaces.name", "Name")}</th>
              <th>{t("shell.spaces.description", "Description")}</th>
              <th>{t("shell.spaces.goal", "Primary goal")}</th>
              <th>{t("shell.spaces.default_agents", "Default agents")}</th>
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
      </section>
    </div>
  );
}
