import { useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import {
  loadRunDecisions,
  loadRunGates,
  loadRunStages,
  loadRunTasks,
  type DecisionSnapshot,
  type GateRecord,
  type RunStageEvent,
  type Task,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function RunDetailPage() {
  const { runId } = useParams({ from: "/shell/runs/$runId" });
  const [stages, setStages] = useState<RunStageEvent[]>([]);
  const [decisions, setDecisions] = useState<DecisionSnapshot[]>([]);
  const [gates, setGates] = useState<GateRecord[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [error, setError] = useState<string | null>(null);
  const { formatTime } = useUiHelpers();

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      loadRunStages(runId),
      loadRunDecisions(runId),
      loadRunGates(runId),
      loadRunTasks(runId),
    ])
      .then(([s, d, g, t]) => {
        if (cancelled) return;
        setStages(s);
        setDecisions(d);
        setGates(g);
        setTasks(t);
      })
      .catch((err) => {
        if (!cancelled) setError(String(err));
      });
    return () => {
      cancelled = true;
    };
  }, [runId]);

  if (error) return <div className="page"><p className="error">Failed: {error}</p></div>;

  return (
    <div className="page">
      <h1>Run {runId.slice(0, 16)}</h1>

      <section>
        <h2>Stage timeline</h2>
        {stages.length === 0 ? (
          <p className="muted">(none)</p>
        ) : (
          <ul className="timeline">
            {stages.map((s) => (
              <li key={s.id} className="timeline__item">
                <time>{formatTime(s.at)}</time>
                <div>
                  <span className={`stage stage--${s.to_stage}`}>{s.to_stage}</span>
                  {s.from_stage && <small> (from {s.from_stage})</small>}
                  <br />
                  <code>{s.policy_rule_id ?? "—"}</code> — {s.reason ?? "—"}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section>
        <h2>Tasks</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Agent</th>
              <th>Title</th>
              <th>Kind</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {tasks.map((t) => (
              <tr key={t.id}>
                <td>{t.assigned_agent_id}</td>
                <td>{t.title}</td>
                <td>{t.task_kind}</td>
                <td><span className={`pill pill--${t.status}`}>{t.status}</span></td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section>
        <h2>Decisions</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Stage</th>
              <th>Next action</th>
              <th>Rule</th>
              <th>At</th>
            </tr>
          </thead>
          <tbody>
            {decisions.map((d) => (
              <tr key={d.id}>
                <td>{d.stage}</td>
                <td><code>{d.next_action}</code></td>
                <td><code>{d.policy_rule_id}</code></td>
                <td>{formatTime(d.at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section>
        <h2>Gates</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Gate</th>
              <th>Verdict</th>
              <th>Reason</th>
            </tr>
          </thead>
          <tbody>
            {gates.map((g) => (
              <tr key={g.id}>
                <td>{g.gate_name}</td>
                <td><span className={`pill pill--${g.verdict}`}>{g.verdict}</span></td>
                <td>{g.reason ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
