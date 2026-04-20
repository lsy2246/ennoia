import { Link } from "@tanstack/react-router";

import { PageHeader } from "@/components/PageHeader";
import { useWorkspaceSnapshot } from "@/hooks/useWorkspaceSnapshot";
import { useUiHelpers } from "@/stores/ui";

export function WorkflowsPage() {
  const { snapshot, loading, error, refresh } = useWorkspaceSnapshot();
  const { t, formatDateTime } = useUiHelpers();

  if (loading || !snapshot) {
    return <div className="page"><p>{t("shell.loading.workflows", "Loading workflows…")}</p></div>;
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.workflows.title", "Workflows")}
        description={t(
          "shell.page.workflows.description",
          "Track orchestrated runs, stages and task execution across conversations.",
        )}
        meta={[`${snapshot.runs.length} ${t("shell.meta.total", "total")}`]}
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
              <th>{t("shell.workflows.conversation", "Conversation")}</th>
              <th>{t("shell.workflows.lane", "Lane")}</th>
              <th>{t("shell.workflows.stage", "Stage")}</th>
              <th>{t("shell.workflows.goal", "Goal")}</th>
              <th>{t("shell.workflows.created_at", "Created")}</th>
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
                <td><span className={`stage stage--${run.stage}`}>{run.stage}</span></td>
                <td>{run.goal}</td>
                <td>{formatDateTime(run.created_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
