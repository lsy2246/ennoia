import { useEffect, useMemo, useState } from "react";
import type { ExtensionUiRenderHelpers } from "@ennoia/ui-sdk";

import {
  getWorkflowRunDetail,
  getWorkflowWorkspaceSummary,
  listWorkflowRuns,
  type WorkflowArtifact,
  type WorkflowDecisionSnapshot,
  type WorkflowGateVerdict,
  type WorkflowRun,
  type WorkflowRunDetail,
  type WorkflowStageEvent,
  type WorkflowTask,
  type WorkflowWorkspaceSummary,
} from "./api";
import "./workflow.css";

const STAGE_ORDER = [
  "pending",
  "planning",
  "dispatched",
  "running",
  "reviewing",
  "completed",
] as const;

type WorkflowPageProps = {
  helpers: ExtensionUiRenderHelpers;
};

type InspectorSelection =
  | { kind: "run" }
  | { kind: "stage"; value: string }
  | { kind: "gate"; value: string }
  | { kind: "task"; value: string }
  | { kind: "artifact"; value: string }
  | { kind: "decision"; value: string };

function readInitialSearchParams() {
  if (typeof window === "undefined") {
    return { runId: "", conversationId: "" };
  }
  const params = new URLSearchParams(window.location.search);
  return {
    runId: params.get("run_id") ?? "",
    conversationId: params.get("conversation_id") ?? "",
  };
}

function updateLocationSearch(runId: string, conversationId: string) {
  if (typeof window === "undefined") {
    return;
  }
  const url = new URL(window.location.href);
  if (runId) {
    url.searchParams.set("run_id", runId);
  } else {
    url.searchParams.delete("run_id");
  }
  if (conversationId) {
    url.searchParams.set("conversation_id", conversationId);
  } else {
    url.searchParams.delete("conversation_id");
  }
  window.history.replaceState(null, "", url);
}

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

function severityBadgeClass(value: string) {
  if (value === "completed" || value === "allow" || value === "info") {
    return "workflow-badge is-success";
  }
  if (value === "blocked" || value === "warn") {
    return "workflow-badge is-warn";
  }
  if (value === "failed" || value === "deny" || value === "cancelled") {
    return "workflow-badge is-danger";
  }
  if (value === "running" || value === "planning" || value === "dispatched" || value === "reviewing") {
    return "workflow-badge is-active";
  }
  return "workflow-badge";
}

function safeJson(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function latestStageReason(stage: string, stageEvents: WorkflowStageEvent[]) {
  for (let index = stageEvents.length - 1; index >= 0; index -= 1) {
    const event = stageEvents[index];
    if (event.to_stage === stage) {
      return event.reason ?? "";
    }
  }
  return "";
}

function latestDecisionForStage(stage: string, decisions: WorkflowDecisionSnapshot[]) {
  for (let index = decisions.length - 1; index >= 0; index -= 1) {
    if (decisions[index]?.stage === stage) {
      return decisions[index];
    }
  }
  return null;
}

export default function WorkflowPage({ helpers }: WorkflowPageProps) {
  const { formatDateTime, t } = helpers;
  const initial = useMemo(() => readInitialSearchParams(), []);
  const [workspace, setWorkspace] = useState<WorkflowWorkspaceSummary | null>(null);
  const [runs, setRuns] = useState<WorkflowRun[]>([]);
  const [detail, setDetail] = useState<WorkflowRunDetail | null>(null);
  const [selectedRunId, setSelectedRunId] = useState(initial.runId);
  const [inspector, setInspector] = useState<InspectorSelection>({ kind: "run" });
  const [query, setQuery] = useState("");
  const [stageFilter, setStageFilter] = useState("all");
  const [conversationId, setConversationId] = useState(initial.conversationId);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => {
      void refresh(false);
    }, 8000);
    return () => window.clearInterval(timer);
  }, [conversationId, query, stageFilter]);

  useEffect(() => {
    updateLocationSearch(selectedRunId, conversationId);
  }, [conversationId, selectedRunId]);

  useEffect(() => {
    if (!selectedRunId) {
      setDetail(null);
      return;
    }
    void loadDetail(selectedRunId);
  }, [selectedRunId]);

  async function refresh(showBusy = true) {
    if (showBusy) {
      setBusy(true);
    }
    setError(null);
    try {
      const [nextWorkspace, nextRuns] = await Promise.all([
        getWorkflowWorkspaceSummary(),
        listWorkflowRuns({
          conversation_id: conversationId.trim() || undefined,
          stage: stageFilter === "all" ? undefined : stageFilter,
          q: query.trim() || undefined,
          limit: 120,
        }),
      ]);
      setWorkspace(nextWorkspace);
      setRuns(nextRuns);
      if (!selectedRunId && nextRuns[0]) {
        setSelectedRunId(nextRuns[0].id);
      } else if (selectedRunId && !nextRuns.some((item) => item.id === selectedRunId) && nextRuns[0]) {
        setSelectedRunId(nextRuns[0].id);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      if (showBusy) {
        setBusy(false);
      }
    }
  }

  async function loadDetail(runId: string) {
    try {
      const nextDetail = await getWorkflowRunDetail(runId);
      setDetail(nextDetail);
    } catch (err) {
      setError(String(err));
    }
  }

  const visitedStages = useMemo(
    () => new Set(detail?.stage_events.map((item) => item.to_stage) ?? []),
    [detail?.stage_events],
  );
  const decisions = detail?.decisions ?? [];
  const branchStage = detail && !STAGE_ORDER.includes(detail.run.stage as typeof STAGE_ORDER[number])
    ? detail.run.stage
    : null;

  const inspectorContent = useMemo(() => {
    if (!detail) {
      return null;
    }
    if (inspector.kind === "stage") {
      const decision = latestDecisionForStage(inspector.value, decisions);
      return (
        <div className="workflow-inspector-body">
          <article className="workflow-flow-card">
            <span className="workflow-card__eyebrow">Stage</span>
            <h3 className="workflow-inspector-title">{stageLabel(inspector.value)}</h3>
            <p>{latestStageReason(inspector.value, detail.stage_events) || "当前阶段还没有记录原因。"}</p>
            {decision ? <pre className="workflow-json">{safeJson(decision)}</pre> : null}
          </article>
        </div>
      );
    }
    if (inspector.kind === "gate") {
      const gate = detail.gate_verdicts.find((item) => item.gate_name === inspector.value);
      if (!gate) {
        return null;
      }
      return (
        <div className="workflow-inspector-body">
          <article className="workflow-flow-card">
            <span className="workflow-card__eyebrow">Gate</span>
            <h3 className="workflow-inspector-title">{gate.gate_name}</h3>
            <div className="workflow-tag-row">
              <span className={severityBadgeClass(gate.severity)}>{gate.severity}</span>
              <span className={severityBadgeClass(gate.allow ? "allow" : "deny")}>{gate.allow ? "allow" : "deny"}</span>
            </div>
            <p>{gate.reason}</p>
            <pre className="workflow-json">{safeJson(gate)}</pre>
          </article>
        </div>
      );
    }
    if (inspector.kind === "task") {
      const task = detail.tasks.find((item) => item.id === inspector.value);
      if (!task) {
        return null;
      }
      return (
        <div className="workflow-inspector-body">
          <article className="workflow-flow-card">
            <span className="workflow-card__eyebrow">Task</span>
            <h3 className="workflow-inspector-title">{task.title}</h3>
            <div className="workflow-tag-row">
              <span className={severityBadgeClass(task.status)}>{task.status}</span>
              <span className="workflow-badge">{task.task_kind}</span>
              <span className="workflow-badge">{task.assigned_agent_id}</span>
            </div>
            <pre className="workflow-json">{safeJson(task)}</pre>
          </article>
        </div>
      );
    }
    if (inspector.kind === "artifact") {
      const artifact = detail.artifacts.find((item) => item.id === inspector.value);
      if (!artifact) {
        return null;
      }
      return (
        <div className="workflow-inspector-body">
          <article className="workflow-flow-card">
            <span className="workflow-card__eyebrow">Artifact</span>
            <h3 className="workflow-inspector-title">{artifact.kind}</h3>
            <p>{artifact.relative_path}</p>
            <pre className="workflow-json">{safeJson(artifact)}</pre>
          </article>
        </div>
      );
    }
    if (inspector.kind === "decision") {
      const decision = decisions.find((item) => item.id === inspector.value);
      if (!decision) {
        return null;
      }
      return (
        <div className="workflow-inspector-body">
          <article className="workflow-flow-card">
            <span className="workflow-card__eyebrow">Decision</span>
            <h3 className="workflow-inspector-title">{decision.next_action}</h3>
            <p>{decision.policy_rule_id}</p>
            <pre className="workflow-json">{safeJson(decision)}</pre>
          </article>
        </div>
      );
    }
    return (
      <div className="workflow-inspector-body">
        <article className="workflow-flow-card">
          <span className="workflow-card__eyebrow">Run</span>
          <h3 className="workflow-inspector-title">{detail.run.goal}</h3>
          <div className="workflow-tag-row">
            <span className={severityBadgeClass(detail.run.stage)}>{detail.run.stage}</span>
            <span className="workflow-badge">{detail.run.trigger}</span>
            <span className="workflow-badge">{detail.run.owner.kind}:{detail.run.owner.id}</span>
          </div>
          <pre className="workflow-json">{safeJson(detail.run)}</pre>
        </article>
        <article className="workflow-flow-card">
          <span className="workflow-card__eyebrow">Stage Timeline</span>
          <div className="workflow-toolbar">
            {detail.stage_events.length === 0 ? (
              <div className="workflow-empty"><p>还没有 stage event。</p></div>
            ) : (
              detail.stage_events.map((event) => (
                <button
                  key={event.id}
                  type="button"
                  className="workflow-mini-card"
                  onClick={() => setInspector({ kind: "stage", value: event.to_stage })}
                >
                  <div className="workflow-meta-row">
                    <span className={severityBadgeClass(event.to_stage)}>{stageLabel(event.to_stage)}</span>
                    <span className="workflow-badge">{formatDateTime(event.at)}</span>
                  </div>
                  <p>{event.reason ?? "无额外原因"}</p>
                </button>
              ))
            )}
          </div>
        </article>
      </div>
    );
  }, [decisions, detail, formatDateTime, inspector]);

  return (
    <div className="workflow-shell">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("ext.workflow.eyebrow", "Workflow")}</span>
          <h1>{t("ext.workflow.title", "工作编排工作台")}</h1>
          <p>{t("ext.workflow.description", "把 run 从计划、判定、分派、执行到产出串成一条可视化流程，直接看到它为什么走到这一步。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="workflow-summary-grid">
          <article className="workflow-metric">
            <span>Total Runs</span>
            <strong>{workspace?.runs_total ?? 0}</strong>
            <small>全部 workflow 实例</small>
          </article>
          <article className="workflow-metric">
            <span>Active</span>
            <strong>{workspace?.runs_active ?? 0}</strong>
            <small>仍在推进中的 run</small>
          </article>
          <article className="workflow-metric">
            <span>Blocked</span>
            <strong>{workspace?.runs_blocked ?? 0}</strong>
            <small>需要人工或上下文解除阻塞</small>
          </article>
          <article className="workflow-metric">
            <span>Decisions</span>
            <strong>{workspace?.decisions_total ?? 0}</strong>
            <small>已记录的决策快照</small>
          </article>
          <article className="workflow-metric">
            <span>Artifacts</span>
            <strong>{workspace?.artifacts_total ?? 0}</strong>
            <small>所有输出产物</small>
          </article>
        </div>
      </section>

      <div className="workflow-grid">
        <section className="work-panel workflow-sidebar">
          <div className="workflow-toolbar">
            <div className="page-heading">
              <span>{t("ext.workflow.catalog", "Run Catalog")}</span>
              <h1>{t("ext.workflow.catalog_title", "运行目录")}</h1>
              <p>{t("ext.workflow.catalog_description", "先定位 run，再进入中间的流程图和右侧检查器。")}</p>
            </div>
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("ext.workflow.search_placeholder", "搜索 run id、goal、conversation")}
            />
            <div className="workflow-input-row">
              <input
                value={conversationId}
                onChange={(event) => setConversationId(event.target.value)}
                placeholder={t("ext.workflow.conversation_filter", "可选：conversation id")}
              />
              <select value={stageFilter} onChange={(event) => setStageFilter(event.target.value)}>
                <option value="all">{t("ext.workflow.filter_all", "全部阶段")}</option>
                {["pending", "planning", "dispatched", "running", "reviewing", "completed", "blocked", "failed", "cancelled"].map((stage) => (
                  <option key={stage} value={stage}>{stageLabel(stage)}</option>
                ))}
              </select>
            </div>
            <div className="button-row">
              <button type="button" onClick={() => void refresh()} disabled={busy}>
                {busy ? t("ext.workflow.loading", "加载中…") : t("ext.workflow.refresh", "刷新")}
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => {
                  setConversationId("");
                  setQuery("");
                  setStageFilter("all");
                }}
              >
                {t("ext.workflow.reset", "清空筛选")}
              </button>
            </div>
          </div>
          <div className="workflow-list">
            {runs.length === 0 ? (
              <div className="workflow-empty">
                <p>{t("ext.workflow.empty", "当前还没有 workflow run。")}</p>
              </div>
            ) : (
              runs.map((run) => (
                <button
                  key={run.id}
                  type="button"
                  className={`workflow-run-card ${run.id === selectedRunId ? "is-active" : ""}`}
                  onClick={() => {
                    setSelectedRunId(run.id);
                    setInspector({ kind: "run" });
                  }}
                >
                  <header>
                    <div>
                      <span className="workflow-card__eyebrow">{run.id}</span>
                      <h3>{run.goal}</h3>
                    </div>
                    <span className={severityBadgeClass(run.stage)}>{run.stage}</span>
                  </header>
                  <p>{run.conversation_id}</p>
                  <div className="workflow-meta-row">
                    <span className="workflow-badge">{run.trigger}</span>
                    <span className="workflow-badge">{formatDateTime(run.updated_at)}</span>
                  </div>
                </button>
              ))
            )}
          </div>
        </section>

        <section className="work-panel workflow-canvas">
          <div className="page-heading">
            <span>{t("ext.workflow.canvas", "Flow Canvas")}</span>
            <h1>{detail?.run.goal ?? t("ext.workflow.canvas_empty_title", "选中一个 run 查看流程")}</h1>
            <p>{detail ? `${detail.run.id} · ${detail.run.conversation_id}` : t("ext.workflow.canvas_empty", "左侧选中 run 后，这里会展示阶段主干、gate、task 和 artifact。")}</p>
          </div>

          {!detail ? (
            <div className="workflow-empty">
              <p>{t("ext.workflow.canvas_empty", "左侧选中 run 后，这里会展示阶段主干、gate、task 和 artifact。")}</p>
            </div>
          ) : (
            <div className="workflow-flow">
              <div className="workflow-stage-rail">
                {STAGE_ORDER.map((stage) => {
                  const isVisited = visitedStages.has(stage) || detail.run.stage === stage;
                  const isActive = detail.run.stage === stage;
                  const isPending = !isVisited;
                  return (
                    <div
                      key={stage}
                      className={[
                        "workflow-stage-node",
                        isVisited ? "is-completed" : "",
                        isActive ? "is-active" : "",
                        isPending ? "is-pending" : "",
                      ].join(" ").trim()}
                    >
                      <button type="button" onClick={() => setInspector({ kind: "stage", value: stage })}>
                        <span className="workflow-card__eyebrow">Stage</span>
                        <strong>{stageLabel(stage)}</strong>
                        <small>{latestStageReason(stage, detail.stage_events) || "等待进入"}</small>
                      </button>
                    </div>
                  );
                })}
                {branchStage ? (
                  <div className="workflow-stage-node is-branch is-active">
                    <button type="button" onClick={() => setInspector({ kind: "stage", value: branchStage })}>
                      <span className="workflow-card__eyebrow">Branch</span>
                      <strong>{stageLabel(branchStage)}</strong>
                      <small>{latestStageReason(branchStage, detail.stage_events) || "分支状态"}</small>
                    </button>
                  </div>
                ) : null}
              </div>

              <div className="workflow-flow-grid">
                <article className="workflow-flow-card">
                  <header>
                    <div>
                      <span className="workflow-card__eyebrow">Gate Lane</span>
                      <h3>{t("ext.workflow.gates", "判定关卡")}</h3>
                    </div>
                    <span className="workflow-badge">{detail.gate_verdicts.length}</span>
                  </header>
                  <div className="workflow-toolbar">
                    {detail.gate_verdicts.length === 0 ? (
                      <div className="workflow-empty"><p>当前没有 gate verdict。</p></div>
                    ) : (
                      detail.gate_verdicts.map((gate) => (
                        <button
                          key={gate.gate_name}
                          type="button"
                          className="workflow-mini-card"
                          onClick={() => setInspector({ kind: "gate", value: gate.gate_name })}
                        >
                          <div className="workflow-tag-row">
                            <span className={severityBadgeClass(gate.severity)}>{gate.severity}</span>
                            <span className={severityBadgeClass(gate.allow ? "allow" : "deny")}>{gate.allow ? "allow" : "deny"}</span>
                          </div>
                          <h3>{gate.gate_name}</h3>
                          <p>{gate.reason}</p>
                        </button>
                      ))
                    )}
                  </div>
                </article>

                <article className="workflow-flow-card">
                  <header>
                    <div>
                      <span className="workflow-card__eyebrow">Decision Lane</span>
                      <h3>{t("ext.workflow.decisions", "决策快照")}</h3>
                    </div>
                    <span className="workflow-badge">{decisions.length}</span>
                  </header>
                  <div className="workflow-toolbar">
                    {decisions.length === 0 ? (
                      <div className="workflow-empty"><p>当前没有 decision snapshot。</p></div>
                    ) : (
                      decisions.map((decision) => (
                        <button
                          key={decision.id}
                          type="button"
                          className="workflow-mini-card"
                          onClick={() => setInspector({ kind: "decision", value: decision.id })}
                        >
                          <div className="workflow-tag-row">
                            <span className="workflow-badge">{decision.stage}</span>
                            <span className={severityBadgeClass("running")}>{decision.next_action}</span>
                          </div>
                          <p>{decision.policy_rule_id}</p>
                        </button>
                      ))
                    )}
                  </div>
                </article>

                <article className="workflow-flow-card">
                  <header>
                    <div>
                      <span className="workflow-card__eyebrow">Execution Lane</span>
                      <h3>{t("ext.workflow.tasks", "任务分支")}</h3>
                    </div>
                    <span className="workflow-badge">{detail.tasks.length}</span>
                  </header>
                  <div className="workflow-toolbar">
                    {detail.tasks.length === 0 ? (
                      <div className="workflow-empty"><p>当前没有 task。</p></div>
                    ) : (
                      detail.tasks.map((task) => (
                        <button
                          key={task.id}
                          type="button"
                          className="workflow-mini-card"
                          onClick={() => setInspector({ kind: "task", value: task.id })}
                        >
                          <div className="workflow-tag-row">
                            <span className={severityBadgeClass(task.status)}>{task.status}</span>
                            <span className="workflow-badge">{task.assigned_agent_id}</span>
                          </div>
                          <h3>{task.title}</h3>
                          <p>{task.task_kind}</p>
                        </button>
                      ))
                    )}
                  </div>
                </article>

                <article className="workflow-flow-card">
                  <header>
                    <div>
                      <span className="workflow-card__eyebrow">Output Lane</span>
                      <h3>{t("ext.workflow.artifacts", "产物输出")}</h3>
                    </div>
                    <span className="workflow-badge">{detail.artifacts.length}</span>
                  </header>
                  <div className="workflow-toolbar">
                    {detail.artifacts.length === 0 ? (
                      <div className="workflow-empty"><p>当前还没有 artifact。</p></div>
                    ) : (
                      detail.artifacts.map((artifact) => (
                        <button
                          key={artifact.id}
                          type="button"
                          className="workflow-mini-card"
                          onClick={() => setInspector({ kind: "artifact", value: artifact.id })}
                        >
                          <div className="workflow-tag-row">
                            <span className="workflow-badge">{artifact.kind}</span>
                            <span className="workflow-badge">{formatDateTime(artifact.created_at)}</span>
                          </div>
                          <p>{artifact.relative_path}</p>
                        </button>
                      ))
                    )}
                  </div>
                </article>
              </div>
            </div>
          )}
        </section>

        <section className="work-panel workflow-inspector">
          <div className="page-heading">
            <span>{t("ext.workflow.inspector", "Inspector")}</span>
            <h1>{t("ext.workflow.inspector_title", "节点检查器")}</h1>
            <p>{t("ext.workflow.inspector_description", "点中流程上的 stage、gate、task 或 artifact，这里就显示它的原因和结构化详情。")}</p>
          </div>
          {inspectorContent ?? (
            <div className="workflow-empty">
              <p>{t("ext.workflow.inspector_empty", "还没有可展示的节点详情。")}</p>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
