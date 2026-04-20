import { useEffect, useMemo, useState } from "react";

import { createJob } from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useWorkspaceSnapshot } from "@/hooks/useWorkspaceSnapshot";
import { useUiHelpers } from "@/stores/ui";

export function JobsPage() {
  const { snapshot, loading, error, refresh } = useWorkspaceSnapshot();
  const { t, formatDateTime } = useUiHelpers();
  const [ownerKind, setOwnerKind] = useState<"global" | "space" | "agent">("global");
  const [ownerId, setOwnerId] = useState("workspace");
  const [jobKind, setJobKind] = useState("maintenance");
  const [scheduleKind, setScheduleKind] = useState("interval");
  const [scheduleValue, setScheduleValue] = useState("300");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const ownerOptions = useMemo(() => {
    if (!snapshot) {
      return [{ value: "workspace", label: "workspace" }];
    }
    if (ownerKind === "space") {
      return snapshot.spaces.map((space) => ({
        value: space.id,
        label: `${space.display_name} (${space.id})`,
      }));
    }
    if (ownerKind === "agent") {
      return snapshot.agents.map((agent) => ({
        value: agent.id,
        label: `${agent.display_name} (${agent.id})`,
      }));
    }
    return [{ value: "workspace", label: "workspace" }];
  }, [ownerKind, snapshot]);

  useEffect(() => {
    setOwnerId(ownerOptions[0]?.value ?? "workspace");
  }, [ownerOptions]);

  async function handleCreateJob() {
    setBusy(true);
    setSubmitError(null);
    setMessage(null);
    try {
      await createJob({
        owner_kind: ownerKind,
        owner_id: ownerId,
        job_kind: jobKind,
        schedule_kind: scheduleKind,
        schedule_value: scheduleValue,
      });
      setMessage(t("shell.jobs.created", "Job created and scheduled."));
      await refresh();
    } catch (err) {
      setSubmitError(String(err));
    } finally {
      setBusy(false);
    }
  }

  if (loading || !snapshot) {
    return <div className="page"><p>{t("shell.loading.jobs", "Loading jobs…")}</p></div>;
  }

  const pendingJobs = snapshot.jobs.filter((job) => job.status === "pending").length;
  const runningJobs = snapshot.jobs.filter((job) => job.status === "running").length;

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.jobs.title", "Jobs")}
        description={t(
          "shell.page.jobs.description",
          "View and create scheduled work so the timing mechanism is visible and manageable.",
        )}
        meta={[
          `${snapshot.jobs.length} ${t("shell.meta.total", "total")}`,
          `${pendingJobs} ${t("shell.jobs.pending", "pending")}`,
          `${runningJobs} ${t("shell.jobs.running", "running")}`,
        ]}
        actions={
          <button className="secondary" onClick={() => void refresh()}>
            {t("shell.action.refresh", "Refresh")}
          </button>
        }
      />

      {error && <div className="error">{error}</div>}
      {submitError && <div className="error">{submitError}</div>}
      {message && <div className="success">{message}</div>}

      <section className="surface-card">
        <h2>{t("shell.jobs.create", "Create scheduled job")}</h2>
        <div className="form-row">
          <label>
            {t("shell.jobs.owner_kind", "Owner kind")}
            <select
              value={ownerKind}
              onChange={(event) => setOwnerKind(event.target.value as "global" | "space" | "agent")}
            >
              <option value="global">{t("shell.owner.global", "Global")}</option>
              <option value="space">{t("shell.owner.space", "Space")}</option>
              <option value="agent">{t("shell.owner.agent", "Agent")}</option>
            </select>
          </label>
          <label>
            {t("shell.jobs.owner_id", "Owner")}
            <select value={ownerId} onChange={(event) => setOwnerId(event.target.value)}>
              {ownerOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="form-row">
          <label>
            {t("shell.jobs.kind", "Job kind")}
            <select value={jobKind} onChange={(event) => setJobKind(event.target.value)}>
              <option value="maintenance">maintenance</option>
              <option value="distill_episodes">distill_episodes</option>
              <option value="compute_embedding">compute_embedding</option>
              <option value="retire_expired">retire_expired</option>
            </select>
          </label>
          <label>
            {t("shell.jobs.schedule_kind", "Schedule kind")}
            <select value={scheduleKind} onChange={(event) => setScheduleKind(event.target.value)}>
              <option value="once">once</option>
              <option value="delay">delay</option>
              <option value="interval">interval</option>
              <option value="cron">cron</option>
            </select>
          </label>
          <label>
            {t("shell.jobs.schedule_value", "Schedule value")}
            <input value={scheduleValue} onChange={(event) => setScheduleValue(event.target.value)} />
          </label>
        </div>
        <div className="actions">
          <button onClick={handleCreateJob} disabled={busy}>
            {busy ? t("shell.action.creating", "Creating…") : t("shell.jobs.create_action", "Create job")}
          </button>
        </div>
      </section>

      <section>
        <h2>{t("shell.jobs.list", "Scheduled jobs")}</h2>
        <table className="table">
          <thead>
            <tr>
              <th>ID</th>
              <th>{t("shell.jobs.owner", "Owner")}</th>
              <th>{t("shell.jobs.kind", "Job kind")}</th>
              <th>{t("shell.jobs.schedule", "Schedule")}</th>
              <th>{t("shell.jobs.next_run", "Next run")}</th>
              <th>{t("shell.jobs.status", "Status")}</th>
              <th>{t("shell.jobs.created_at", "Created")}</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.jobs.map((job) => (
              <tr key={job.id}>
                <td><code>{job.id.slice(0, 12)}</code></td>
                <td><code>{job.owner_kind}/{job.owner_id}</code></td>
                <td>{job.job_kind}</td>
                <td><code>{job.schedule_kind}:{job.schedule_value}</code></td>
                <td>{job.next_run_at ? formatDateTime(job.next_run_at) : "—"}</td>
                <td><span className={`pill pill--${job.status}`}>{job.status}</span></td>
                <td>{formatDateTime(job.created_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
