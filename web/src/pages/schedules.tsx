import { useEffect, useMemo, useState, type FormEvent } from "react";

import {
  createSchedule,
  deleteSchedule,
  listAgents,
  listChatLanes,
  listChats,
  listSchedules,
  pauseSchedule,
  resumeSchedule,
  runSchedule,
  updateSchedule,
  type AgentProfile,
  type ChatLane,
  type ChatThread,
  type ScheduleExecutor,
  type SchedulePayload,
  type ScheduleRecord,
  type ScheduleTrigger,
} from "@ennoia/api-client";
import { Select } from "@/components/Select";
import { useUiHelpers } from "@/stores/ui";

type TriggerKind = ScheduleTrigger["kind"];
type ExecutorKind = ScheduleExecutor["kind"];
type AgentRunMode = "independent" | "conversation";

type ScheduleFormState = {
  name: string;
  description: string;
  triggerKind: TriggerKind;
  onceAt: string;
  intervalSeconds: string;
  cronExpression: string;
  cronNextRunAt: string;
  executorKind: ExecutorKind;
  commandText: string;
  commandCwd: string;
  commandTimeoutMs: string;
  agentId: string;
  agentPrompt: string;
  modelId: string;
  maxTurns: string;
  agentRunMode: AgentRunMode;
  contextConversationId: string;
  deliveryConversationId: string;
  deliveryLaneId: string;
  contentMode: "full" | "summary" | "conclusion";
  retryAttempts: string;
  retryBackoffSeconds: string;
  enabled: boolean;
};

function fromDateTimeLocal(value: string) {
  return new Date(value).toISOString();
}

function toDateTimeLocal(value?: string | null) {
  if (!value) {
    return "";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "";
  }
  const offset = date.getTimezoneOffset();
  return new Date(date.getTime() - offset * 60_000).toISOString().slice(0, 16);
}

function formatScheduleTime(value?: string | null) {
  if (!value) {
    return "—";
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

function formatJson(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function statusBadgeClass(status?: string | null) {
  if (status === "completed" || status === "completed_with_warning") {
    return "badge--success";
  }
  if (status === "failed") {
    return "badge--danger";
  }
  return "badge--muted";
}

function summarizeText(value: string, max = 72) {
  const trimmed = value.trim();
  if (trimmed.length <= max) {
    return trimmed;
  }
  return `${trimmed.slice(0, max)}…`;
}

function scheduleTitle(schedule: ScheduleRecord) {
  if (schedule.name?.trim()) {
    return schedule.name;
  }
  if (schedule.executor.kind === "command") {
    return summarizeText(schedule.executor.command.command, 48);
  }
  return `Agent · ${schedule.executor.agent.agent_id}`;
}

function describeTrigger(trigger: ScheduleTrigger) {
  switch (trigger.kind) {
    case "once":
      return `once · ${formatScheduleTime(trigger.at)}`;
    case "cron":
      return `cron · ${trigger.expression}`;
    case "interval":
      return `interval · ${trigger.every_seconds}s`;
  }
}

function describeExecutor(executor: ScheduleExecutor) {
  if (executor.kind === "command") {
    return summarizeText(executor.command.command, 64) || "command";
  }
  const model = executor.agent.model_id?.trim() ? ` · ${executor.agent.model_id}` : "";
  return `${executor.agent.agent_id}${model}`;
}

function describeDeliveryMode(value?: string | null) {
  switch (value) {
    case "summary":
      return "摘要";
    case "conclusion":
      return "最终结论";
    case "full":
    default:
      return "完整结果";
  }
}

function createDefaultForm(agents: AgentProfile[]): ScheduleFormState {
  const defaultAgentId = agents.find((item) => item.enabled)?.id ?? agents[0]?.id ?? "";
  return {
    name: "",
    description: "",
    triggerKind: "interval",
    onceAt: "",
    intervalSeconds: "3600",
    cronExpression: "0 9 * * *",
    cronNextRunAt: "",
    executorKind: "command",
    commandText: "",
    commandCwd: "",
    commandTimeoutMs: "120000",
    agentId: defaultAgentId,
    agentPrompt: "",
    modelId: "",
    maxTurns: "",
    agentRunMode: "independent",
    contextConversationId: "",
    deliveryConversationId: "",
    deliveryLaneId: "",
    contentMode: "full",
    retryAttempts: "1",
    retryBackoffSeconds: "0",
    enabled: true,
  };
}

export function Schedules() {
  const { t } = useUiHelpers();
  const [schedules, setSchedules] = useState<ScheduleRecord[]>([]);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [conversations, setConversations] = useState<ChatThread[]>([]);
  const [lanesByConversation, setLanesByConversation] = useState<Record<string, ChatLane[]>>({});
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<ScheduleFormState>(() => createDefaultForm([]));
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void hydrate();
  }, []);

  const agentOptions = useMemo(() => {
    const options = agents.map((agent) => ({
      value: agent.id,
      label: `${agent.display_name} · ${agent.id}`,
    }));
    if (form.agentId && !options.some((item) => item.value === form.agentId)) {
      options.unshift({ value: form.agentId, label: `${form.agentId} · ${t("web.common.unknown", "未知")}` });
    }
    return options;
  }, [agents, form.agentId, t]);

  const agentContextConversationOptions = useMemo(() => {
    const options = [
      { value: "", label: t("web.schedules.context_none", "不指定") },
      ...conversations.map((conversation) => ({
        value: conversation.id,
        label: `${conversation.title} · ${conversation.id}`,
      })),
    ];
    for (const conversationId of [form.contextConversationId]) {
      if (conversationId && !options.some((item) => item.value === conversationId)) {
        options.push({
          value: conversationId,
          label: `${conversationId} · ${t("web.common.unknown", "未知")}`,
        });
      }
    }
    return options;
  }, [conversations, form.contextConversationId, t]);

  const deliveryConversationOptions = useMemo(() => {
    const options = [
      { value: "", label: t("web.schedules.delivery_none", "不投递") },
      ...conversations.map((conversation) => ({
        value: conversation.id,
        label: `${conversation.title} · ${conversation.id}`,
      })),
    ];
    for (const conversationId of [form.deliveryConversationId]) {
      if (conversationId && !options.some((item) => item.value === conversationId)) {
        options.push({
          value: conversationId,
          label: `${conversationId} · ${t("web.common.unknown", "未知")}`,
        });
      }
    }
    return options;
  }, [conversations, form.deliveryConversationId, t]);

  const conversationMap = useMemo(
    () => new Map(conversations.map((conversation) => [conversation.id, conversation.title])),
    [conversations],
  );

  const deliveryLaneOptions = useMemo(() => {
    const activeLanes = form.deliveryConversationId
      ? (lanesByConversation[form.deliveryConversationId] ?? [])
      : [];
    const options = [
      { value: "", label: t("web.schedules.lane_default", "默认 lane") },
      ...activeLanes.map((lane) => ({
        value: lane.id,
        label: `${lane.name} · ${lane.id}`,
      })),
    ];
    if (form.deliveryLaneId && !options.some((item) => item.value === form.deliveryLaneId)) {
      options.push({
        value: form.deliveryLaneId,
        label: `${form.deliveryLaneId} · ${t("web.common.unknown", "未知")}`,
      });
    }
    return options;
  }, [form.deliveryConversationId, form.deliveryLaneId, lanesByConversation, t]);

  const selectedAgent = agents.find((item) => item.id === form.agentId);

  async function hydrate() {
    setError(null);
    try {
      const [nextSchedules, nextAgents, nextConversations] = await Promise.all([
        listSchedules(),
        listAgents(),
        listChats(),
      ]);
      setSchedules(nextSchedules);
      setAgents(nextAgents);
      setConversations(nextConversations);
      setForm((current) => {
        if (current.agentId) {
          return current;
        }
        return {
          ...current,
          agentId: nextAgents.find((item) => item.enabled)?.id ?? nextAgents[0]?.id ?? "",
        };
      });
    } catch (err) {
      setError(String(err));
    }
  }

  async function ensureConversationLanes(conversationId: string) {
    if (!conversationId.trim() || lanesByConversation[conversationId]) {
      return;
    }
    const lanes = await listChatLanes(conversationId);
    setLanesByConversation((current) => ({
      ...current,
      [conversationId]: lanes,
    }));
  }

  function updateForm(patch: Partial<ScheduleFormState>) {
    setForm((current) => ({ ...current, ...patch }));
  }

  function resetForm(nextAgents = agents) {
    setEditingId(null);
    setForm(createDefaultForm(nextAgents));
  }

  async function loadSchedule(schedule: ScheduleRecord) {
    const nextForm = createDefaultForm(agents);
    nextForm.name = schedule.name ?? "";
    nextForm.description = schedule.description ?? "";
    nextForm.enabled = schedule.enabled;
    nextForm.deliveryConversationId = schedule.delivery?.conversation_id ?? "";
    nextForm.deliveryLaneId = schedule.delivery?.lane_id ?? "";
    nextForm.contentMode = schedule.delivery?.content_mode ?? "full";
    nextForm.retryAttempts = String(schedule.retry?.max_attempts ?? 1);
    nextForm.retryBackoffSeconds = String(schedule.retry?.backoff_seconds ?? 0);

    switch (schedule.trigger.kind) {
      case "once":
        nextForm.triggerKind = "once";
        nextForm.onceAt = toDateTimeLocal(schedule.trigger.at);
        break;
      case "cron":
        nextForm.triggerKind = "cron";
        nextForm.cronExpression = schedule.trigger.expression;
        nextForm.cronNextRunAt = toDateTimeLocal(schedule.trigger.next_run_at);
        break;
      case "interval":
        nextForm.triggerKind = "interval";
        nextForm.intervalSeconds = String(schedule.trigger.every_seconds);
        break;
    }

    if (schedule.executor.kind === "command") {
      nextForm.executorKind = "command";
      nextForm.commandText = schedule.executor.command.command;
      nextForm.commandCwd = schedule.executor.command.cwd ?? "";
      nextForm.commandTimeoutMs = String(schedule.executor.command.timeout_ms ?? 120000);
    } else {
      nextForm.executorKind = "agent";
      nextForm.agentId = schedule.executor.agent.agent_id;
      nextForm.agentPrompt = schedule.executor.agent.prompt;
      nextForm.modelId = schedule.executor.agent.model_id ?? "";
      nextForm.agentRunMode = schedule.executor.agent.context?.conversation_id
        ? "conversation"
        : "independent";
      nextForm.contextConversationId =
        schedule.executor.agent.context?.conversation_id ?? "";
      nextForm.maxTurns = schedule.executor.agent.max_turns
        ? String(schedule.executor.agent.max_turns)
        : "";
    }

    setEditingId(schedule.id);
    setForm(nextForm);
    setMessage(null);
    setError(null);
    const conversationsToLoad = [nextForm.deliveryConversationId].filter(
      (value, index, items) => value && items.indexOf(value) === index,
    );
    if (conversationsToLoad.length) {
      try {
        await Promise.all(conversationsToLoad.map((conversationId) => ensureConversationLanes(conversationId)));
      } catch (err) {
        setError(String(err));
      }
    }
  }

  function buildTrigger(): ScheduleTrigger {
    if (form.triggerKind === "once") {
      if (!form.onceAt) {
        throw new Error(t("web.schedules.once_required", "请选择一次性触发时间。"));
      }
      return { kind: "once", at: fromDateTimeLocal(form.onceAt) };
    }

    if (form.triggerKind === "cron") {
      if (!form.cronNextRunAt) {
        throw new Error(t("web.schedules.cron_next_required", "请填写 cron 的下一次触发时间。"));
      }
      return {
        kind: "cron",
        expression: form.cronExpression.trim() || "* * * * *",
        next_run_at: fromDateTimeLocal(form.cronNextRunAt),
      };
    }

    return {
      kind: "interval",
      every_seconds: Math.max(1, Number(form.intervalSeconds) || 1),
    };
  }

  function buildExecutor(): ScheduleExecutor {
    if (form.executorKind === "command") {
      if (!form.commandText.trim()) {
        throw new Error(t("web.schedules.command_required", "请输入要运行的命令。"));
      }
      return {
        kind: "command",
        command: {
          command: form.commandText.trim(),
          cwd: form.commandCwd.trim() || null,
          timeout_ms: Math.max(1000, Number(form.commandTimeoutMs) || 120000),
        },
      };
    }

    if (!form.agentId.trim()) {
      throw new Error(t("web.schedules.agent_required", "请选择要触发的 Agent。"));
    }
    if (!form.agentPrompt.trim()) {
      throw new Error(t("web.schedules.agent_prompt_required", "请输入要交给 Agent 的任务内容。"));
    }
    if (form.agentRunMode === "conversation" && !form.contextConversationId.trim()) {
      throw new Error(t("web.schedules.context_conversation_required", "请选择参考哪个会话运行。"));
    }
    return {
      kind: "agent",
      agent: {
        agent_id: form.agentId.trim(),
        prompt: form.agentPrompt.trim(),
        model_id: form.modelId.trim() || null,
        max_turns: form.maxTurns.trim() ? Math.max(1, Number(form.maxTurns) || 1) : null,
        context: {
          conversation_id:
            form.agentRunMode === "conversation" ? form.contextConversationId.trim() || null : null,
        },
      },
    };
  }

  function buildPayload(): SchedulePayload {
    return {
      name: form.name.trim() || null,
      description: form.description.trim() || null,
      trigger: buildTrigger(),
      executor: buildExecutor(),
      delivery: {
        conversation_id: form.deliveryConversationId.trim() || null,
        lane_id: form.deliveryLaneId.trim() || null,
        content_mode: form.deliveryConversationId.trim() ? form.contentMode : null,
      },
      retry: {
        max_attempts: Math.max(1, Number(form.retryAttempts) || 1),
        backoff_seconds: Math.max(0, Number(form.retryBackoffSeconds) || 0),
      },
      owner: { kind: "operator", id: "local" },
      enabled: form.enabled,
    };
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const payload = buildPayload();
      if (editingId) {
        await updateSchedule(editingId, payload);
      } else {
        await createSchedule(payload);
      }
      await hydrate();
      resetForm();
      setMessage(
        editingId
          ? t("web.schedules.saved", "定时器已保存。")
          : t("web.schedules.created", "定时器已创建。"),
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function runAction(action: () => Promise<unknown>, successMessage: string) {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      await action();
      await hydrate();
      setMessage(successMessage);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="resource-layout resource-layout--single">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.schedules.eyebrow", "Schedules")}</span>
          <h1>{t("web.schedules.title", "定时器按计划触发命令或 Agent。")}</h1>
          <p>
            {t(
              "web.schedules.description",
              "系统负责保存计划、计算到期、运行命令或触发 Agent，并可把结果投递到某个会话。",
            )}
          </p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        {message ? <div className="success">{message}</div> : null}

        <form className="editor-form" onSubmit={handleSubmit}>
          <div className="form-grid">
            <label>
              {t("web.schedules.name", "名称")}
              <input
                value={form.name}
                onChange={(event) => updateForm({ name: event.target.value })}
                placeholder={t("web.schedules.name_placeholder", "例如：每天早上同步日报")}
              />
            </label>
            <label>
              {t("web.schedules.executor", "执行方式")}
              <Select
                value={form.executorKind}
                onChange={(value) => updateForm({ executorKind: value as ExecutorKind })}
                options={[
                  { value: "command", label: t("web.schedules.executor_command", "运行命令") },
                  { value: "agent", label: t("web.schedules.executor_agent", "让 Agent 执行") },
                ]}
              />
            </label>
            <label>
              {t("web.schedules.trigger", "触发器")}
              <Select
                value={form.triggerKind}
                onChange={(value) => updateForm({ triggerKind: value as TriggerKind })}
                options={[
                  { value: "interval", label: t("web.schedules.interval", "间隔") },
                  { value: "once", label: t("web.schedules.once", "一次性") },
                  { value: "cron", label: t("web.schedules.cron", "Cron") },
                ]}
              />
            </label>
          </div>

          <label>
            {t("web.schedules.description_label", "说明")}
            <input
              value={form.description}
              onChange={(event) => updateForm({ description: event.target.value })}
              placeholder={t("web.schedules.description_placeholder", "可选：补充这个定时器的用途")}
            />
          </label>

          {form.triggerKind === "interval" ? (
            <label>
              {t("web.schedules.every_seconds", "间隔秒数")}
              <input
                value={form.intervalSeconds}
                inputMode="numeric"
                onChange={(event) => updateForm({ intervalSeconds: event.target.value })}
              />
            </label>
          ) : null}

          {form.triggerKind === "once" ? (
            <label>
              {t("web.schedules.once_at", "触发时间")}
              <input
                type="datetime-local"
                value={form.onceAt}
                onChange={(event) => updateForm({ onceAt: event.target.value })}
              />
            </label>
          ) : null}

          {form.triggerKind === "cron" ? (
            <div className="form-grid">
              <label>
                {t("web.schedules.cron_expression", "Cron 表达式")}
                <input
                  value={form.cronExpression}
                  onChange={(event) => updateForm({ cronExpression: event.target.value })}
                />
              </label>
              <label>
                {t("web.schedules.cron_next_run", "下一次触发时间")}
                <input
                  type="datetime-local"
                  value={form.cronNextRunAt}
                  onChange={(event) => updateForm({ cronNextRunAt: event.target.value })}
                />
              </label>
            </div>
          ) : null}

          {form.executorKind === "command" ? (
            <div className="stack">
              <label>
                {t("web.schedules.command", "命令")}
                <textarea
                  value={form.commandText}
                  onChange={(event) => updateForm({ commandText: event.target.value })}
                  rows={4}
                  placeholder={t("web.schedules.command_placeholder", "例如 bun run --cwd web build")}
                />
              </label>
              <div className="form-grid">
                <label>
                  {t("web.schedules.command_cwd", "工作目录")}
                  <input
                    value={form.commandCwd}
                    onChange={(event) => updateForm({ commandCwd: event.target.value })}
                    placeholder={t("web.schedules.command_cwd_placeholder", "可选，默认使用服务进程目录")}
                  />
                </label>
                <label>
                  {t("web.schedules.command_timeout", "超时毫秒")}
                  <input
                    value={form.commandTimeoutMs}
                    inputMode="numeric"
                    onChange={(event) => updateForm({ commandTimeoutMs: event.target.value })}
                  />
                </label>
              </div>
            </div>
          ) : (
            <div className="stack">
              <div className="form-grid">
                <label>
                  {t("web.schedules.agent", "Agent")}
                  <Select
                    value={form.agentId}
                    onChange={(value) => updateForm({ agentId: value })}
                    options={agentOptions}
                    placeholder={t("web.schedules.agent_placeholder", "请选择 Agent")}
                  />
                </label>
                <label>
                  {t("web.schedules.model", "模型")}
                  <input
                    value={form.modelId}
                    onChange={(event) => updateForm({ modelId: event.target.value })}
                    placeholder={selectedAgent?.model_id || selectedAgent?.default_model || t("web.schedules.model_placeholder", "留空则跟随 Agent 默认模型")}
                  />
                </label>
              </div>
              <div className="form-grid">
                <label>
                  {t("web.schedules.max_turns", "最大轮数")}
                  <input
                    value={form.maxTurns}
                    inputMode="numeric"
                    onChange={(event) => updateForm({ maxTurns: event.target.value })}
                    placeholder={t("web.schedules.max_turns_placeholder", "可选")}
                  />
                </label>
                <label>
                  {t("web.schedules.agent_run_mode", "运行方式")}
                  <Select
                    value={form.agentRunMode}
                    onChange={(value) =>
                      updateForm({
                        agentRunMode: value as AgentRunMode,
                        contextConversationId: value === "conversation" ? form.contextConversationId : "",
                      })
                    }
                    options={[
                      { value: "independent", label: t("web.schedules.agent_run_mode_independent", "独立运行") },
                      { value: "conversation", label: t("web.schedules.agent_run_mode_conversation", "参考某个会话") },
                    ]}
                  />
                </label>
              </div>
              {form.agentRunMode === "conversation" ? (
                <label>
                  {t("web.schedules.context_conversation", "运行参考会话")}
                  <Select
                    value={form.contextConversationId}
                    onChange={(value) => updateForm({ contextConversationId: value })}
                    options={agentContextConversationOptions}
                  />
                </label>
              ) : null}
              <label>
                {t("web.schedules.agent_prompt", "任务内容")}
                <textarea
                  value={form.agentPrompt}
                  onChange={(event) => updateForm({ agentPrompt: event.target.value })}
                  rows={6}
                  placeholder={t("web.schedules.agent_prompt_placeholder", "例如：整理今天的待办并输出一条晨会提醒")}
                />
              </label>
            </div>
          )}

          <label>
            {t("web.schedules.delivery", "投递到会话")}
            <Select
              value={form.deliveryConversationId}
              onChange={(value) => {
                updateForm({ deliveryConversationId: value, deliveryLaneId: "" });
                if (value) {
                  void ensureConversationLanes(value);
                }
              }}
              options={deliveryConversationOptions}
            />
          </label>

          {form.deliveryConversationId ? (
            <div className="form-grid">
                <label>
                  {t("web.schedules.delivery_lane", "投递到 lane")}
                  <Select
                    value={form.deliveryLaneId}
                    onChange={(value) => updateForm({ deliveryLaneId: value })}
                    options={deliveryLaneOptions}
                  />
                </label>
              <label>
                {t("web.schedules.delivery_content", "投递内容")}
                <Select
                  value={form.contentMode}
                  onChange={(value) => updateForm({ contentMode: value as ScheduleFormState["contentMode"] })}
                  options={[
                    { value: "full", label: t("web.schedules.delivery_content_full", "完整结果") },
                    { value: "summary", label: t("web.schedules.delivery_content_summary", "摘要") },
                    { value: "conclusion", label: t("web.schedules.delivery_content_conclusion", "最终结论") },
                  ]}
                />
              </label>
            </div>
          ) : null}

          <div className="form-grid">
            <label>
              {t("web.schedules.retry_attempts", "失败重试次数")}
              <input
                value={form.retryAttempts}
                inputMode="numeric"
                onChange={(event) => updateForm({ retryAttempts: event.target.value })}
              />
            </label>
            <label>
              {t("web.schedules.retry_backoff", "重试间隔秒数")}
              <input
                value={form.retryBackoffSeconds}
                inputMode="numeric"
                onChange={(event) => updateForm({ retryBackoffSeconds: event.target.value })}
              />
            </label>
          </div>

          <div className="button-row">
            <button type="submit" disabled={busy}>
              {editingId
                ? t("web.schedules.save", "保存修改")
                : t("web.schedules.create", "创建定时器")}
            </button>
            {editingId ? (
              <button type="button" className="secondary" disabled={busy} onClick={() => resetForm()}>
                {t("web.schedules.cancel_edit", "取消编辑")}
              </button>
            ) : null}
            <button type="button" className="secondary" onClick={() => void hydrate()}>
              {t("web.action.refresh", "刷新")}
            </button>
          </div>
        </form>
      </section>

      <section className="work-panel">
        <div className="panel-title">{t("web.schedules.list", "定时器列表")}</div>
        <div className="stack">
          {schedules.length === 0 ? (
            <div className="empty-card">{t("web.schedules.empty", "还没有定时器。")}</div>
          ) : (
            schedules.map((schedule) => {
              const contextConversationId =
                schedule.executor.kind === "agent"
                  ? schedule.executor.agent.context?.conversation_id
                  : null;
              const deliveryConversationId = schedule.delivery?.conversation_id;
              const deliveryLaneId = schedule.delivery?.lane_id;
              return (
                <article key={schedule.id} className="resource-card">
                  <header>
                    <strong>{scheduleTitle(schedule)}</strong>
                    <span className={`badge ${schedule.enabled ? "badge--success" : "badge--muted"}`}>
                      {schedule.enabled
                        ? t("web.common.enabled", "启用")
                        : t("web.common.disabled", "停用")}
                    </span>
                  </header>
                  <p>{schedule.id}</p>
                  {schedule.description ? <p className="helper-text">{schedule.description}</p> : null}
                  <div className="kv-list">
                    <span>{t("web.schedules.executor", "执行方式")}</span>
                    <strong>
                      {schedule.executor.kind === "command"
                        ? t("web.schedules.executor_command", "运行命令")
                        : t("web.schedules.executor_agent", "让 Agent 执行")}
                    </strong>
                    <span>{t("web.schedules.target", "目标")}</span>
                    <strong>{describeExecutor(schedule.executor)}</strong>
                    <span>{t("web.schedules.agent_run_mode", "运行方式")}</span>
                    <strong>
                      {schedule.executor.kind === "agent"
                        ? contextConversationId
                          ? t("web.schedules.agent_run_mode_conversation", "参考某个会话")
                          : t("web.schedules.agent_run_mode_independent", "独立运行")
                        : "—"}
                    </strong>
                    <span>{t("web.schedules.context_conversation", "运行参考会话")}</span>
                    <strong>
                      {contextConversationId
                        ? conversationMap.get(contextConversationId)
                          ? `${conversationMap.get(contextConversationId)} · ${contextConversationId}`
                          : contextConversationId
                        : "—"}
                    </strong>
                    <span>{t("web.schedules.delivery", "投递到会话")}</span>
                    <strong>
                      {deliveryConversationId
                        ? conversationMap.get(deliveryConversationId)
                          ? `${conversationMap.get(deliveryConversationId)} · ${deliveryConversationId}`
                          : deliveryConversationId
                        : "—"}
                    </strong>
                    <span>{t("web.schedules.delivery_lane", "投递到 lane")}</span>
                    <strong>{deliveryLaneId || "—"}</strong>
                    <span>{t("web.schedules.delivery_content", "投递内容")}</span>
                    <strong>
                      {deliveryConversationId
                        ? describeDeliveryMode(schedule.delivery?.content_mode)
                        : "—"}
                    </strong>
                    <span>{t("web.schedules.trigger", "触发器")}</span>
                    <strong>{describeTrigger(schedule.trigger)}</strong>
                    <span>{t("web.schedules.next_run", "下次触发")}</span>
                    <strong>{formatScheduleTime(schedule.next_run_at)}</strong>
                    <span>{t("web.schedules.last_run", "上次触发")}</span>
                    <strong>{formatScheduleTime(schedule.last_run_at)}</strong>
                    <span>{t("web.schedules.retry", "重试")}</span>
                    <strong>
                      {(schedule.retry?.max_attempts ?? 1)} / {(schedule.retry?.backoff_seconds ?? 0)}s
                    </strong>
                    <span>{t("web.common.status", "状态")}</span>
                    <strong>
                      <span className={`badge ${statusBadgeClass(schedule.last_status)}`}>
                        {schedule.last_status ?? "—"}
                      </span>
                    </strong>
                    <span>{t("web.schedules.last_error", "最近错误")}</span>
                    <strong>
                      {schedule.last_error ? (
                        <span className="badge badge--danger">{schedule.last_error}</span>
                      ) : (
                        "—"
                      )}
                    </strong>
                  </div>

                  {schedule.history?.length ? (
                    <div className="stack">
                      <div className="panel-title">{t("web.schedules.history", "最近运行")}</div>
                      {schedule.history.map((run) => (
                        <details key={run.id} className="resource-card resource-card--subtle">
                          <summary>
                            <strong>{formatScheduleTime(run.finished_at)}</strong>
                          </summary>
                          <div className="kv-list">
                            <span>{t("web.common.status", "状态")}</span>
                            <strong>
                              <span className={`badge ${statusBadgeClass(run.status)}`}>{run.status}</span>
                            </strong>
                            <span>{t("web.schedules.started_at", "开始时间")}</span>
                            <strong>{formatScheduleTime(run.started_at)}</strong>
                            <span>{t("web.schedules.finished_at", "完成时间")}</span>
                            <strong>{formatScheduleTime(run.finished_at)}</strong>
                            <span>{t("web.schedules.attempt", "尝试")}</span>
                            <strong>{run.attempt}</strong>
                            <span>{t("web.schedules.delivered", "已投递")}</span>
                            <strong>{run.delivered ? t("web.common.yes", "是") : t("web.common.no", "否")}</strong>
                          </div>
                          {run.error ? <div className="error">{run.error}</div> : null}
                          {!run.error && run.delivery_error ? (
                            <div className="error">{run.delivery_error}</div>
                          ) : null}
                          <label>
                            {t("web.schedules.run_output", "运行输出")}
                            <textarea readOnly rows={12} value={formatJson(run.output)} />
                          </label>
                        </details>
                      ))}
                    </div>
                  ) : null}

                  <div className="button-row">
                    <button type="button" className="secondary" disabled={busy} onClick={() => void loadSchedule(schedule)}>
                      {t("web.action.edit", "编辑")}
                    </button>
                    <button
                      type="button"
                      className="secondary"
                      disabled={busy}
                      onClick={() =>
                        void runAction(
                          () => runSchedule(schedule.id),
                          t("web.schedules.ran", "定时器已手动运行。"),
                        )}
                    >
                      {t("web.schedules.run_now", "立即运行")}
                    </button>
                    <button
                      type="button"
                      className="secondary"
                      disabled={busy}
                      onClick={() =>
                        void runAction(
                          () => (schedule.enabled ? pauseSchedule(schedule.id) : resumeSchedule(schedule.id)),
                          schedule.enabled
                            ? t("web.schedules.paused", "定时器已暂停。")
                            : t("web.schedules.resumed", "定时器已恢复。"),
                        )}
                    >
                      {schedule.enabled
                        ? t("web.schedules.pause", "暂停")
                        : t("web.schedules.resume", "恢复")}
                    </button>
                    <button
                      type="button"
                      className="danger"
                      disabled={busy}
                      onClick={() =>
                        void runAction(
                          () => deleteSchedule(schedule.id),
                          t("web.schedules.deleted", "定时器已删除。"),
                        )}
                    >
                      {t("web.action.delete", "删除")}
                    </button>
                  </div>
                </article>
              );
            })
          )}
        </div>
      </section>
    </div>
  );
}
