import { useEffect, useMemo, useState } from "react";
import type { ExtensionUiRenderHelpers } from "@ennoia/ui-sdk";

import {
  createWorkflowStream,
  parseWorkflowStreamPayload,
  type WorkflowRun,
  type WorkflowRunDetail,
  type WorkflowStreamSnapshot,
} from "../page/api";

type WorkflowConversationCardProps = {
  conversationId: string;
  helpers: ExtensionUiRenderHelpers;
};

function stageLabel(stage: string) {
  switch (stage) {
    case "pending":
      return "Pending";
    case "planning":
      return "Planning";
    case "dispatched":
      return "Dispatched";
    case "running":
      return "Running";
    case "reviewing":
      return "Reviewing";
    case "completed":
      return "Completed";
    case "blocked":
      return "Blocked";
    case "failed":
      return "Failed";
    case "cancelled":
      return "Cancelled";
    default:
      return stage;
  }
}

function badgeClass(value: string) {
  if (value === "completed" || value === "allow" || value === "info" || value === "done") {
    return "workflow-badge is-success";
  }
  if (value === "blocked" || value === "warn" || value === "planning" || value === "pending") {
    return "workflow-badge is-warn";
  }
  if (value === "failed" || value === "deny" || value === "cancelled") {
    return "workflow-badge is-danger";
  }
  return "workflow-badge is-active";
}

function summarizeGoal(value: string) {
  const normalized = value.replace(/\s+/g, " ").trim();
  if (normalized.length <= 56) {
    return normalized;
  }
  return `${normalized.slice(0, 56)}…`;
}

export default function WorkflowConversationCard({
  conversationId,
  helpers,
}: WorkflowConversationCardProps) {
  const { formatDateTime, t } = helpers;
  const [runs, setRuns] = useState<WorkflowRun[]>([]);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [detail, setDetail] = useState<WorkflowRunDetail | null>(null);
  const [loadingRuns, setLoadingRuns] = useState(false);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectedRun = useMemo(
    () => runs.find((item) => item.id === selectedRunId) ?? runs[0] ?? null,
    [runs, selectedRunId],
  );

  useEffect(() => {
    if (typeof EventSource === "undefined") {
      return;
    }

    setLoadingRuns(true);
    setLoadingDetail(Boolean(selectedRunId));
    const stream = createWorkflowStream({
      conversation_id: conversationId,
      run_id: selectedRunId ?? undefined,
      limit: 24,
    });

    const applySnapshot = (snapshot: WorkflowStreamSnapshot) => {
      setRuns(snapshot.runs);
      setDetail(snapshot.detail ?? null);
      setSelectedRunId((current) =>
        current && snapshot.runs.some((item) => item.id === current) ? current : snapshot.runs[0]?.id ?? null);
      setLoadingRuns(false);
      setLoadingDetail(false);
      setError(null);
    };

    const handleSnapshot = (event: Event) => {
      if (!(event instanceof MessageEvent) || typeof event.data !== "string") {
        return;
      }
      applySnapshot(parseWorkflowStreamPayload(event.data));
    };

    const handleErrorEvent = (event: Event) => {
      if (!(event instanceof MessageEvent) || typeof event.data !== "string") {
        return;
      }
      try {
        const payload = JSON.parse(event.data) as { message?: string };
        setError(payload.message?.trim() || t("ext.workflow.stream_error", "工作流实时同步暂时受阻，正在自动重试。"));
      } catch {
        setError(t("ext.workflow.stream_error", "工作流实时同步暂时受阻，正在自动重试。"));
      } finally {
        setLoadingRuns(false);
        setLoadingDetail(false);
      }
    };

    stream.addEventListener("workflow.snapshot", handleSnapshot);
    stream.addEventListener("workflow.error", handleErrorEvent);
    stream.onopen = () => {
      setLoadingRuns(false);
      setLoadingDetail(false);
      setError(null);
    };
    stream.onerror = () => {
      setLoadingRuns(false);
      setLoadingDetail(false);
      setError(t("ext.workflow.stream_error", "工作流实时同步暂时受阻，正在自动重试。"));
    };

    return () => {
      stream.removeEventListener("workflow.snapshot", handleSnapshot);
      stream.removeEventListener("workflow.error", handleErrorEvent);
      stream.close();
    };
  }, [conversationId, selectedRunId, t]);

  if (!loadingRuns && runs.length === 0 && !error) {
    return null;
  }

  return (
    <section className="workflow-inline-card">
      <div className="workflow-inline-card__header">
        <div className="workflow-inline-card__copy">
          <span>{t("ext.workflow.session.eyebrow", "Workflow")}</span>
          <h2>{t("ext.workflow.session.title", "计划执行")}</h2>
          <p>{t("ext.workflow.session.description", "只在当前会话确实产生了 plan/run 时出现。你可以直接在会话里看它怎么推进、卡在哪里、产出了什么。")}</p>
        </div>
        <span className="workflow-inline-card__count">{runs.length}</span>
      </div>

      {runs.length > 0 ? (
        <div className="workflow-inline-strip">
          {runs.map((run) => (
            <button
              key={run.id}
              type="button"
              className={run.id === selectedRun?.id ? "workflow-inline-chip is-active" : "workflow-inline-chip"}
              onClick={() => setSelectedRunId(run.id)}
            >
              <strong>{summarizeGoal(run.goal)}</strong>
              <span>{run.id}</span>
              <div className="workflow-tag-row">
                <span className={badgeClass(run.stage)}>{stageLabel(run.stage)}</span>
                <span className="workflow-badge">{run.trigger}</span>
              </div>
            </button>
          ))}
        </div>
      ) : null}

      {error ? (
        <div className="workflow-empty workflow-empty--danger">{error}</div>
      ) : loadingRuns || loadingDetail ? (
        <div className="workflow-empty">{t("ext.workflow.loading", "加载中…")}</div>
      ) : detail ? (
        <div className="workflow-inline-detail">
          <div className="workflow-inline-summary">
            <div>
              <span className="workflow-card__eyebrow">{t("ext.workflow.session.current", "当前计划")}</span>
              <h3>{detail.run.goal}</h3>
              <p>{detail.run.id} · {detail.run.conversation_id}</p>
            </div>
            <div className="workflow-tag-row">
              <span className={badgeClass(detail.run.stage)}>{stageLabel(detail.run.stage)}</span>
              <span className="workflow-badge">{detail.run.trigger}</span>
              <span className="workflow-badge">{formatDateTime(detail.run.updated_at)}</span>
            </div>
          </div>

          <div className="workflow-inline-grid">
            <article className="workflow-panel-card">
              <div className="workflow-card__eyebrow">{t("ext.workflow.session.timeline", "执行轨迹")}</div>
              <div className="workflow-panel-body">
                {detail.stage_events.length === 0 ? (
                  <div className="workflow-empty">{t("ext.workflow.session.timeline_empty", "还没有 stage event。")}</div>
                ) : (
                  detail.stage_events.map((event) => (
                    <div key={event.id} className="workflow-inline-item">
                      <div className="workflow-inline-item__top">
                        <span className={badgeClass(event.to_stage)}>
                          {event.from_stage ? `${stageLabel(event.from_stage)} -> ${stageLabel(event.to_stage)}` : stageLabel(event.to_stage)}
                        </span>
                        <small>{formatDateTime(event.at)}</small>
                      </div>
                      <p>{event.reason ?? t("ext.workflow.session.timeline_reason", "没有额外说明。")}</p>
                    </div>
                  ))
                )}
              </div>
            </article>

            <article className="workflow-panel-card">
              <div className="workflow-card__eyebrow">{t("ext.workflow.session.tasks", "任务清单")}</div>
              <div className="workflow-panel-body">
                {detail.tasks.length === 0 ? (
                  <div className="workflow-empty">{t("ext.workflow.session.tasks_empty", "这次计划还没有拆出任务。")}</div>
                ) : (
                  detail.tasks.map((task) => (
                    <div key={task.id} className="workflow-inline-item">
                      <strong>{task.title}</strong>
                      <div className="workflow-tag-row">
                        <span className={badgeClass(task.status)}>{task.status}</span>
                        <span className="workflow-badge">{task.assigned_agent_id}</span>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </article>

            <article className="workflow-panel-card">
              <div className="workflow-card__eyebrow">{t("ext.workflow.session.artifacts", "产物输出")}</div>
              <div className="workflow-panel-body">
                {detail.artifacts.length === 0 ? (
                  <div className="workflow-empty">{t("ext.workflow.session.artifacts_empty", "这次计划还没有产物。")}</div>
                ) : (
                  detail.artifacts.map((artifact) => (
                    <div key={artifact.id} className="workflow-inline-item">
                      <strong>{artifact.kind}</strong>
                      <div className="workflow-tag-row">
                        <span className="workflow-badge">{artifact.relative_path}</span>
                        <span className="workflow-badge">{formatDateTime(artifact.created_at)}</span>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </article>
          </div>
        </div>
      ) : (
        <div className="workflow-empty">{t("ext.workflow.session.empty", "选中一次 run 后，这里会显示它的执行轨迹。")}</div>
      )}
    </section>
  );
}
