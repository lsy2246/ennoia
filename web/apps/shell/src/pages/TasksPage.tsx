import { useEffect, useMemo, useState, type FormEvent } from "react";

import {
  createTaskJob,
  deleteTaskJob,
  disableTaskJob,
  enableTaskJob,
  listAgents,
  listChats,
  listTaskJobs,
  runTaskJobNow,
  type AgentProfile,
  type ChatThread,
  type TaskJob,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function TasksPage() {
  const { t } = useUiHelpers();
  const [jobs, setJobs] = useState<TaskJob[]>([]);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [chats, setChats] = useState<ChatThread[]>([]);
  const [taskType, setTaskType] = useState<"ai_prompt" | "command">("ai_prompt");
  const [ownerId, setOwnerId] = useState("workspace");
  const [scheduleKind, setScheduleKind] = useState("once");
  const [scheduleValue, setScheduleValue] = useState("now");
  const [runAt, setRunAt] = useState("");
  const [prompt, setPrompt] = useState("");
  const [command, setCommand] = useState("");
  const [timeoutSeconds, setTimeoutSeconds] = useState("300");
  const [deleteAfterRun, setDeleteAfterRun] = useState(false);
  const [deliverToConversationId, setDeliverToConversationId] = useState("");
  const [error, setError] = useState<string | null>(null);

  const ownerOptions = useMemo(
    () => [{ id: "workspace", label: t("web.tasks.workspace_runner", "全局工作台") }, ...agents.map((agent) => ({ id: agent.id, label: agent.display_name }))],
    [agents, t],
  );

  const scheduleSummary = useMemo(() => {
    if (scheduleKind === "once") {
      return t("web.tasks.schedule_once_help", "执行一次：`now` 代表立刻执行，也可以配合首次运行时间指定绝对时间。");
    }
    if (scheduleKind === "delay") {
      return t("web.tasks.schedule_delay_help", "延迟执行：规则值填写秒数，例如 `300` 表示 5 分钟后触发。");
    }
    if (scheduleKind === "interval") {
      return t("web.tasks.schedule_interval_help", "循环执行：规则值填写秒数，例如 `3600` 表示每小时执行。");
    }
    return t("web.tasks.schedule_cron_help", "Cron：规则值填写 Cron 表达式，首次运行时间可留空。");
  }, [scheduleKind, t]);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    const [nextJobs, nextAgents, nextChats] = await Promise.all([
      listTaskJobs(),
      listAgents(),
      listChats(),
    ]);
    setJobs(nextJobs);
    setAgents(nextAgents);
    setChats(nextChats);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    try {
      await createTaskJob({
        owner_kind: ownerId === "workspace" ? "global" : "agent",
        owner_id: ownerId,
        job_kind: taskType,
        schedule_kind: scheduleKind,
        schedule_value: scheduleValue,
        run_at: runAt ? new Date(runAt).toISOString() : undefined,
        max_retries: 0,
        payload: {
          task_type: taskType,
          prompt: taskType === "ai_prompt" ? prompt : undefined,
          command: taskType === "command" ? command : undefined,
          timeout_seconds: Number(timeoutSeconds) || undefined,
          delete_after_run: deleteAfterRun,
          deliver_to_conversation_id: deliverToConversationId || undefined,
        },
      });
      setPrompt("");
      setCommand("");
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleJobAction(jobId: string, action: "run" | "delete" | "enable" | "disable") {
    setError(null);
    try {
      if (action === "run") {
        await runTaskJobNow(jobId);
      }
      if (action === "delete") {
        await deleteTaskJob(jobId);
      }
      if (action === "enable") {
        await enableTaskJob(jobId);
      }
      if (action === "disable") {
        await disableTaskJob(jobId);
      }
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="resource-layout">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.tasks.eyebrow", "Task Scheduler")}</span>
          <h1>{t("web.tasks.title", "计划任务分为 AI 任务和命令任务。")}</h1>
          <p>{t("web.tasks.description", "任务可配置超时、运行后删除，以及完成后投递到某个会话。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="stack">
          {jobs.map((job) => (
            <article key={job.id} className="resource-card">
              <header>
                <strong>{job.job_kind}</strong>
                <span>{job.status}</span>
              </header>
              <p>{job.id} · {job.schedule_kind}:{job.schedule_value}</p>
              <div className="tag-row">
                <span>{job.owner_kind}/{job.owner_id}</span>
                <span>{job.next_run_at ?? t("web.tasks.no_next", "暂无下次运行")}</span>
              </div>
              <div className="button-row">
                <button type="button" className="secondary" onClick={() => void handleJobAction(job.id, "run")}>{t("web.tasks.run_now", "立即运行")}</button>
                <button type="button" className="secondary" onClick={() => void handleJobAction(job.id, job.status === "pending" ? "disable" : "enable")}>{job.status === "pending" ? t("web.tasks.pause", "暂停") : t("web.action.enable", "启用")}</button>
                <button type="button" className="danger" onClick={() => void handleJobAction(job.id, "delete")}>{t("web.action.delete", "删除")}</button>
              </div>
            </article>
          ))}
        </div>
      </section>

      <form className="work-panel editor-form" onSubmit={handleSubmit}>
        <div className="panel-title">{t("web.tasks.create", "新建计划任务")}</div>
        <div className="schedule-primer">
          <article className="memory-lane">
            <span>{t("web.tasks.type_ai", "AI Prompt 任务")}</span>
            <strong>AI</strong>
            <small>{t("web.tasks.type_ai_help", "把 Prompt 发给指定执行者，可投递结果回某个会话。")}</small>
          </article>
          <article className="memory-lane">
            <span>{t("web.tasks.type_command", "命令任务")}</span>
            <strong>CMD</strong>
            <small>{t("web.tasks.type_command_help", "执行实际命令链路，适合同步、构建、扫描和维护动作。")}</small>
          </article>
          <article className="memory-lane">
            <span>{t("web.tasks.trigger", "触发方式")}</span>
            <strong>{scheduleKind}</strong>
            <small>{scheduleSummary}</small>
          </article>
        </div>
        <div className="form-grid">
          <label>{t("web.tasks.type", "任务类型")}<select value={taskType} onChange={(event) => setTaskType(event.target.value as "ai_prompt" | "command")}><option value="ai_prompt">{t("web.tasks.type_ai", "AI Prompt 任务")}</option><option value="command">{t("web.tasks.type_command", "命令任务")}</option></select></label>
          <label>{t("web.tasks.runner", "执行者")}<select value={ownerId} onChange={(event) => setOwnerId(event.target.value)}>{ownerOptions.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}</select></label>
          <label>{t("web.tasks.trigger", "触发方式")}<select value={scheduleKind} onChange={(event) => setScheduleKind(event.target.value)}><option value="once">{t("web.tasks.trigger_once", "执行一次")}</option><option value="delay">{t("web.tasks.trigger_delay", "延迟执行")}</option><option value="interval">{t("web.tasks.trigger_interval", "间隔执行")}</option><option value="cron">{t("web.tasks.trigger_cron", "Cron 表达式")}</option></select></label>
          <label>{t("web.tasks.rule", "规则参数")}<input value={scheduleValue} onChange={(event) => setScheduleValue(event.target.value)} /><p className="helper-text">{scheduleSummary}</p></label>
          <label>{t("web.tasks.first_run", "首次运行时间")}<input type="datetime-local" value={runAt} onChange={(event) => setRunAt(event.target.value)} /><p className="helper-text">{t("web.tasks.first_run_help", "可选。留空代表按触发方式计算；需要指定时使用本地日期时间。")}</p></label>
          <label>{t("web.tasks.timeout", "超时秒数")}<input value={timeoutSeconds} onChange={(event) => setTimeoutSeconds(event.target.value)} /></label>
        </div>
        {taskType === "ai_prompt" ? (
          <label>{t("web.tasks.prompt", "Prompt")}<textarea value={prompt} onChange={(event) => setPrompt(event.target.value)} rows={7} required /></label>
        ) : (
          <label>{t("web.tasks.command", "命令")}<textarea value={command} onChange={(event) => setCommand(event.target.value)} rows={7} required /></label>
        )}
        <label>{t("web.tasks.deliver", "完成后投递到会话")}<select value={deliverToConversationId} onChange={(event) => setDeliverToConversationId(event.target.value)}><option value="">{t("web.tasks.no_deliver", "不投递")}</option>{chats.map((chat) => <option key={chat.id} value={chat.id}>{chat.title}</option>)}</select></label>
        <label className="check-row"><input type="checkbox" checked={deleteAfterRun} onChange={(event) => setDeleteAfterRun(event.target.checked)} />{t("web.tasks.delete_after_run", "运行后删除")}</label>
        <button type="submit">{t("web.action.create", "创建")}</button>
      </form>
    </div>
  );
}
