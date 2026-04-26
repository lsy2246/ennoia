import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  ApiError,
  getAgentPermissionPolicy,
  listAgents,
  listPermissionApprovals,
  listPermissionEvents,
  listPermissionPolicySummaries,
  resolvePermissionApproval,
  updateAgentPermissionPolicy,
  type AgentPermissionPolicy,
  type AgentProfile,
  type PermissionApprovalRecord,
  type PermissionEventRecord,
  type PermissionPolicySummary,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

function stringifyJson(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function agentLabel(agents: AgentProfile[], agentId: string) {
  return agents.find((item) => item.id === agentId)?.display_name ?? agentId;
}

export function Permissions() {
  const { formatDateTime, t } = useUiHelpers();
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [summaries, setSummaries] = useState<PermissionPolicySummary[]>([]);
  const [approvals, setApprovals] = useState<PermissionApprovalRecord[]>([]);
  const [events, setEvents] = useState<PermissionEventRecord[]>([]);
  const [selectedAgentId, setSelectedAgentId] = useState<string>("");
  const [policyText, setPolicyText] = useState("");
  const [busy, setBusy] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const selectedAgentIdRef = useRef("");

  const selectedSummary = useMemo(
    () => summaries.find((item) => item.agent_id === selectedAgentId) ?? null,
    [selectedAgentId, summaries],
  );
  const selectedApprovals = useMemo(
    () => approvals.filter((item) => item.agent_id === selectedAgentId),
    [approvals, selectedAgentId],
  );
  const selectedEvents = useMemo(
    () => events.filter((item) => item.agent_id === selectedAgentId),
    [events, selectedAgentId],
  );

  useEffect(() => {
    selectedAgentIdRef.current = selectedAgentId;
  }, [selectedAgentId]);

  const loadPolicy = useCallback(async (agentId: string) => {
    try {
      const policy = await getAgentPermissionPolicy(agentId);
      setPolicyText(stringifyJson(policy));
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const refresh = useCallback(async (preferredAgentId?: string) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const [nextAgents, nextSummaries, nextApprovals, nextEvents] = await Promise.all([
        listAgents(),
        listPermissionPolicySummaries(),
        listPermissionApprovals({ limit: 120 }),
        listPermissionEvents({ limit: 200 }),
      ]);
      setAgents(nextAgents);
      setSummaries(nextSummaries);
      setApprovals(nextApprovals);
      setEvents(nextEvents);
      const fallbackAgentId = preferredAgentId
        ?? selectedAgentIdRef.current
        ?? nextSummaries[0]?.agent_id
        ?? nextAgents[0]?.id
        ?? "";
      const resolvedAgentId = fallbackAgentId || nextSummaries[0]?.agent_id || nextAgents[0]?.id || "";
      setSelectedAgentId(resolvedAgentId);
      if (resolvedAgentId) {
        await loadPolicy(resolvedAgentId);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, [loadPolicy]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!selectedAgentId) {
      setPolicyText("");
      return;
    }
    void loadPolicy(selectedAgentId);
  }, [loadPolicy, selectedAgentId]);

  async function savePolicy() {
    if (!selectedAgentId) {
      return;
    }
    setSaving(true);
    setError(null);
    setNotice(null);
    try {
      const parsed = JSON.parse(policyText) as AgentPermissionPolicy;
      await updateAgentPermissionPolicy(selectedAgentId, parsed);
      setNotice(t("web.permissions.saved", "策略已保存。"));
      await refresh(selectedAgentId);
    } catch (err) {
      if (err instanceof SyntaxError) {
        setError(t("web.permissions.invalid_json", "策略 JSON 无法解析。"));
      } else {
        setError(String(err));
      }
    } finally {
      setSaving(false);
    }
  }

  async function resolveApproval(
    approvalId: string,
    resolution: "allow_once" | "allow_conversation" | "allow_run" | "allow_policy" | "deny",
  ) {
    setError(null);
    setNotice(null);
    try {
      await resolvePermissionApproval(approvalId, resolution);
      setNotice(t("web.permissions.resolved", "审批已处理。"));
      await refresh(selectedAgentId);
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
      } else {
        setError(String(err));
      }
    }
  }

  return (
    <div className="resource-layout">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.permissions.eyebrow", "Agent Permissions")}</span>
          <h1>{t("web.permissions.title", "Agent 权限由系统统一裁决，不交给扩展自行放权。")}</h1>
          <p>{t("web.permissions.description", "这里查看每个 Agent 的策略、待审批请求和最近权限事件。一次授权可以落到单次、当前会话、当前 run 或永久策略。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        {notice ? <div className="success">{notice}</div> : null}
        <div className="button-row">
          <button type="button" className="secondary" onClick={() => void refresh(selectedAgentId)} disabled={busy}>
            {busy ? t("web.common.loading", "加载中") : t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className="card-grid">
          {summaries.map((summary) => (
            <article
              key={summary.agent_id}
              className="resource-card"
              onClick={() => setSelectedAgentId(summary.agent_id)}
            >
              <header>
                <strong>{agentLabel(agents, summary.agent_id)}</strong>
                <span className={`badge ${summary.mode === "default_deny" ? "badge--warn" : "badge--success"}`}>
                  {summary.mode}
                </span>
              </header>
              <p>{summary.agent_id}</p>
              <div className="tag-row">
                <span className="badge badge--success">{summary.allow_count} allow</span>
                <span className="badge badge--warn">{summary.ask_count} ask</span>
                <span className="badge badge--danger">{summary.deny_count} deny</span>
              </div>
            </article>
          ))}
        </div>
      </section>

      <aside className="work-panel editor-form">
        <div className="panel-title">{t("web.permissions.policy", "策略详情")}</div>
        {selectedAgentId ? (
          <>
            <div className="kv-list">
              <span>{t("web.permissions.agent", "Agent")}</span>
              <strong>{agentLabel(agents, selectedAgentId)}</strong>
              <span>ID</span>
              <strong>{selectedAgentId}</strong>
              <span>{t("web.permissions.mode", "默认模式")}</span>
              <strong>{selectedSummary?.mode ?? "default_deny"}</strong>
              <span>{t("web.permissions.pending", "待审批")}</span>
              <strong>{selectedApprovals.filter((item) => item.status === "pending").length}</strong>
            </div>
            <textarea
              value={policyText}
              onChange={(event) => setPolicyText(event.target.value)}
              spellCheck={false}
              style={{
                width: "100%",
                minHeight: 320,
                resize: "vertical",
                borderRadius: 16,
                border: "1px solid var(--border)",
                background: "var(--panel)",
                color: "var(--text)",
                padding: 16,
                fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
                fontSize: 13,
                lineHeight: 1.5,
              }}
            />
            <div className="button-row">
              <button type="button" onClick={() => void savePolicy()} disabled={saving}>
                {saving ? t("web.common.saving", "保存中") : t("web.action.save", "保存")}
              </button>
              <button type="button" className="secondary" onClick={() => void loadPolicy(selectedAgentId)}>
                {t("web.action.reset", "重载")}
              </button>
            </div>

            <div className="stack">
              <div className="panel-title">{t("web.permissions.approvals", "待审批请求")}</div>
              {selectedApprovals.length === 0 ? (
                <div className="empty-card">{t("web.permissions.no_approvals", "当前没有审批记录。")}</div>
              ) : (
                selectedApprovals.map((approval) => (
                  <article key={approval.approval_id} className="mini-card">
                    <strong>{approval.action}</strong>
                    <span>{approval.reason}</span>
                    <span className={`badge ${approval.status === "pending" ? "badge--warn" : approval.status === "approved" ? "badge--success" : "badge--danger"}`}>
                      {approval.status}
                    </span>
                    <span>{formatDateTime(approval.created_at)}</span>
                    <span>{approval.target.kind}:{approval.target.id}</span>
                    <span>{approval.trigger.kind}</span>
                    {approval.status === "pending" ? (
                      <div className="button-row">
                        <button type="button" onClick={() => void resolveApproval(approval.approval_id, "allow_once")}>
                          {t("web.permissions.allow_once", "允许一次")}
                        </button>
                        <button type="button" className="secondary" onClick={() => void resolveApproval(approval.approval_id, "allow_conversation")}>
                          {t("web.permissions.allow_conversation", "允许本会话")}
                        </button>
                        <button type="button" className="secondary" onClick={() => void resolveApproval(approval.approval_id, "allow_run")}>
                          {t("web.permissions.allow_run", "允许本次 run")}
                        </button>
                        <button type="button" className="secondary" onClick={() => void resolveApproval(approval.approval_id, "allow_policy")}>
                          {t("web.permissions.allow_policy", "写入策略")}
                        </button>
                        <button type="button" className="secondary" onClick={() => void resolveApproval(approval.approval_id, "deny")}>
                          {t("web.action.reject", "拒绝")}
                        </button>
                      </div>
                    ) : null}
                  </article>
                ))
              )}
            </div>

            <div className="stack">
              <div className="panel-title">{t("web.permissions.events", "最近权限事件")}</div>
              {selectedEvents.length === 0 ? (
                <div className="empty-card">{t("web.permissions.no_events", "当前没有权限事件。")}</div>
              ) : (
                selectedEvents.slice(0, 24).map((event) => (
                  <article key={event.event_id} className="mini-card">
                    <strong>{event.action}</strong>
                    <span className={`badge ${event.decision === "allow" ? "badge--success" : event.decision === "ask" ? "badge--warn" : "badge--danger"}`}>
                      {event.decision}
                    </span>
                    <span>{event.target.kind}:{event.target.id}</span>
                    <span>{formatDateTime(event.created_at)}</span>
                    <textarea
                      readOnly
                      rows={5}
                      value={stringifyJson({
                        scope: event.scope,
                        matched_rule_id: event.matched_rule_id,
                        approval_id: event.approval_id,
                        trace_id: event.trace_id,
                      })}
                      style={{
                        width: "100%",
                        resize: "vertical",
                        borderRadius: 12,
                        border: "1px solid var(--border)",
                        background: "var(--surface)",
                        color: "var(--text-muted)",
                        padding: 12,
                        fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
                        fontSize: 12,
                        lineHeight: 1.5,
                      }}
                    />
                  </article>
                ))
              )}
            </div>
          </>
        ) : (
          <div className="empty-card">{t("web.permissions.no_agent", "还没有可编辑的 Agent 策略。")}</div>
        )}
      </aside>
    </div>
  );
}
