import type { ExtensionConversationRecord, ExtensionUiRenderHelpers } from "@ennoia/ui-sdk";

type WorkflowConversationRecordProps = {
  record: ExtensionConversationRecord;
  helpers: ExtensionUiRenderHelpers;
};

function asRecord(value: unknown) {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function asStringArray(value: unknown) {
  return Array.isArray(value)
    ? value.map((item) => (typeof item === "string" ? item.trim() : "")).filter(Boolean)
    : [];
}

function statusLabel(value: string | null | undefined) {
  switch ((value ?? "").trim().toLowerCase()) {
    case "ready":
      return "待执行";
    case "running":
      return "执行中";
    case "completed":
      return "已完成";
    case "failed":
      return "失败";
    case "blocked":
      return "已阻塞";
    case "abandoned":
      return "已结束";
    default:
      return value ?? "进行中";
  }
}

function statusClassName(value: string | null | undefined) {
  switch ((value ?? "").trim().toLowerCase()) {
    case "completed":
    case "ready":
      return "badge badge--accent";
    case "failed":
      return "badge badge--danger";
    case "blocked":
      return "badge badge--warn";
    default:
      return "badge";
  }
}

export default function WorkflowConversationRecord({
  record,
  helpers,
}: WorkflowConversationRecordProps) {
  const payload = asRecord(record.payload);
  const steps = asStringArray(payload?.steps).slice(0, 6);
  const goal = typeof payload?.goal === "string" && payload.goal.trim()
    ? payload.goal.trim()
    : record.summary?.trim() || record.title?.trim() || record.kind;
  const revision = typeof payload?.revision === "number" ? payload.revision : null;
  const runId = typeof payload?.run_id === "string" ? payload.run_id.trim() : "";
  const stage = typeof payload?.stage === "string" ? payload.stage.trim() : "";
  const artifacts = Array.isArray(payload?.artifacts) ? payload.artifacts.length : 0;

  return (
    <section className="workflow-inline-detail">
      <div className="workflow-inline-summary">
        <div>
          <span className="workflow-card__eyebrow">
            {record.kind === "workflow.execution" ? "执行过程" : "当前方案"}
          </span>
          <h3>{goal}</h3>
          <p>
            {helpers.formatDateTime(record.updated_at)}
            {revision ? ` · 第 ${revision} 版` : ""}
            {runId ? ` · ${runId}` : ""}
            {stage ? ` · ${stage}` : ""}
          </p>
        </div>
        <div className="workflow-tag-row">
          <span className={statusClassName(record.status)}>{statusLabel(record.status)}</span>
          {artifacts > 0 ? <span className="workflow-badge">{artifacts} 个产物</span> : null}
        </div>
      </div>

      {steps.length > 0 ? (
        <div className="workflow-panel-body">
          {steps.map((step, index) => (
            <div key={`${record.id}:${index}`} className="workflow-inline-item">
              <strong>{index + 1}.</strong>
              <p>{step}</p>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
