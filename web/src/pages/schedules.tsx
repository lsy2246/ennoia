import { useEffect, useMemo, useState, type FormEvent } from "react";

import {
  createSchedule,
  deleteSchedule,
  listScheduleActions,
  listSchedules,
  pauseSchedule,
  resumeSchedule,
  runSchedule,
  type ScheduleRecord,
  type ScheduleTrigger,
} from "@ennoia/api-client";
import type { ExtensionScheduleActionContribution } from "@ennoia/ui-sdk";
import { useUiHelpers } from "@/stores/ui";

type TriggerKind = ScheduleTrigger["kind"];
type ScheduleMode = "ai" | "command";

function fromDateTimeLocal(value: string) {
  return new Date(value).toISOString();
}

function formatScheduleTime(value?: string | null) {
  if (!value) {
    return "—";
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

function actionLabel(
  action: ExtensionScheduleActionContribution,
  resolveText: ReturnType<typeof useUiHelpers>["resolveText"],
) {
  return action.schedule_action.title
    ? resolveText(action.schedule_action.title)
    : `${action.extension_id}:${action.schedule_action.id}`;
}

export function Schedules() {
  const { resolveText, t } = useUiHelpers();
  const [actions, setActions] = useState<ExtensionScheduleActionContribution[]>([]);
  const [schedules, setSchedules] = useState<ScheduleRecord[]>([]);
  const [mode, setMode] = useState<ScheduleMode>("ai");
  const [selectedAction, setSelectedAction] = useState("");
  const [triggerKind, setTriggerKind] = useState<TriggerKind>("interval");
  const [onceAt, setOnceAt] = useState("");
  const [intervalSeconds, setIntervalSeconds] = useState("3600");
  const [cronExpression, setCronExpression] = useState("0 9 * * *");
  const [cronNextRunAt, setCronNextRunAt] = useState("");
  const [paramsText, setParamsText] = useState("{\n  \"goal\": \"定时运行 workflow\"\n}");
  const [commandText, setCommandText] = useState("");
  const [commandCwd, setCommandCwd] = useState("");
  const [commandTimeoutMs, setCommandTimeoutMs] = useState("120000");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const actionOptions = useMemo(
    () =>
      actions.map((action) => ({
        value: `${action.extension_id}:${action.schedule_action.id}`,
        label: actionLabel(action, resolveText),
        action,
      })),
    [actions, resolveText],
  );

  useEffect(() => {
    void hydrate();
  }, []);

  useEffect(() => {
    if (!selectedAction && actionOptions[0]) {
      setSelectedAction(actionOptions[0].value);
    }
  }, [actionOptions, selectedAction]);

  async function hydrate() {
    setError(null);
    try {
      const [nextActions, nextSchedules] = await Promise.all([
        listScheduleActions(),
        listSchedules(),
      ]);
      setActions(nextActions);
      setSchedules(nextSchedules);
    } catch (err) {
      setError(String(err));
    }
  }

  function buildTrigger(): ScheduleTrigger {
    if (triggerKind === "once") {
      if (!onceAt) {
        throw new Error(t("web.schedules.once_required", "请选择一次性触发时间。"));
      }
      return { kind: "once", at: fromDateTimeLocal(onceAt) };
    }
    if (triggerKind === "cron") {
      if (!cronNextRunAt) {
        throw new Error(t("web.schedules.cron_next_required", "请填写 cron 的下一次触发时间。"));
      }
      return {
        kind: "cron",
        expression: cronExpression.trim() || "* * * * *",
        next_run_at: fromDateTimeLocal(cronNextRunAt),
      };
    }
    return {
      kind: "interval",
      every_seconds: Math.max(1, Number(intervalSeconds) || 1),
    };
  }

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const option = actionOptions.find((item) => item.value === selectedAction);
      if (mode === "ai" && !option) {
        throw new Error(t("web.schedules.action_required", "请选择一个定时动作。"));
      }
      if (mode === "command" && !commandText.trim()) {
        throw new Error(t("web.schedules.command_required", "请输入要运行的命令。"));
      }
      const params = mode === "ai" && paramsText.trim() ? JSON.parse(paramsText) : {};
      await createSchedule({
        trigger: buildTrigger(),
        target:
          mode === "ai" && option
            ? {
                kind: "extension",
                extension_id: option.action.extension_id,
                action_id: option.action.schedule_action.id,
              }
            : {
                kind: "command",
                command: {
                  command: commandText.trim(),
                  cwd: commandCwd.trim() || null,
                  timeout_ms: Math.max(1000, Number(commandTimeoutMs) || 120000),
                },
              },
        owner: { kind: "operator", id: "local" },
        params,
        enabled: true,
      });
      await hydrate();
      setMessage(t("web.schedules.created", "定时器已创建。"));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function runAction(
    action: () => Promise<unknown>,
    successMessage: string,
  ) {
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
          <h1>{t("web.schedules.title", "定时器触发扩展声明的动作。")}</h1>
          <p>
            {t(
              "web.schedules.description",
              "系统只负责保存计划、计算到期并调用 Wasm Worker；具体业务语义由扩展的 schedule_actions 定义。",
            )}
          </p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        {message ? <div className="success">{message}</div> : null}

        <form className="editor-form" onSubmit={handleCreate}>
          <div className="form-grid">
            <label>
              {t("web.schedules.mode", "模式")}
              <select
                value={mode}
                onChange={(event) => setMode(event.target.value as ScheduleMode)}
              >
                <option value="ai">{t("web.schedules.mode_ai", "交给 AI 执行")}</option>
                <option value="command">{t("web.schedules.mode_command", "直接运行命令")}</option>
              </select>
              <p className="helper-text">
                {mode === "ai"
                  ? t("web.schedules.mode_ai_help", "按 OpenClaw 风格，到点后把任务发给扩展/AI 工作流。")
                  : t("web.schedules.mode_command_help", "到点后直接在本机 shell 中运行命令，适合脚本和本地自动化。")}
              </p>
            </label>
            <label>
              {t("web.schedules.action", "动作")}
              <select
                value={selectedAction}
                onChange={(event) => setSelectedAction(event.target.value)}
                disabled={mode !== "ai" || actionOptions.length === 0}
              >
                {actionOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t("web.schedules.trigger", "触发器")}
              <select
                value={triggerKind}
                onChange={(event) => setTriggerKind(event.target.value as TriggerKind)}
              >
                <option value="interval">{t("web.schedules.interval", "间隔")}</option>
                <option value="once">{t("web.schedules.once", "一次性")}</option>
                <option value="cron">{t("web.schedules.cron", "Cron")}</option>
              </select>
            </label>
          </div>

          {triggerKind === "interval" ? (
            <label>
              {t("web.schedules.every_seconds", "间隔秒数")}
              <input
                value={intervalSeconds}
                inputMode="numeric"
                onChange={(event) => setIntervalSeconds(event.target.value)}
              />
            </label>
          ) : null}

          {triggerKind === "once" ? (
            <label>
              {t("web.schedules.once_at", "触发时间")}
              <input
                type="datetime-local"
                value={onceAt}
                onChange={(event) => setOnceAt(event.target.value)}
              />
            </label>
          ) : null}

          {triggerKind === "cron" ? (
            <div className="form-grid">
              <label>
                {t("web.schedules.cron_expression", "Cron 表达式")}
                <input
                  value={cronExpression}
                  onChange={(event) => setCronExpression(event.target.value)}
                />
              </label>
              <label>
                {t("web.schedules.cron_next_run", "下一次触发时间")}
                <input
                  type="datetime-local"
                  value={cronNextRunAt}
                  onChange={(event) => setCronNextRunAt(event.target.value)}
                />
              </label>
            </div>
          ) : null}

          {mode === "ai" ? (
            <label>
              {t("web.schedules.params", "参数 JSON")}
              <textarea
                value={paramsText}
                onChange={(event) => setParamsText(event.target.value)}
                rows={6}
              />
            </label>
          ) : (
            <div className="stack">
              <label>
                {t("web.schedules.command", "命令")}
                <textarea
                  value={commandText}
                  onChange={(event) => setCommandText(event.target.value)}
                  rows={4}
                  placeholder={t("web.schedules.command_placeholder", "例如 bun run --cwd web build")}
                />
              </label>
              <div className="form-grid">
                <label>
                  {t("web.schedules.command_cwd", "工作目录")}
                  <input
                    value={commandCwd}
                    onChange={(event) => setCommandCwd(event.target.value)}
                    placeholder={t("web.schedules.command_cwd_placeholder", "可选，默认使用服务进程目录")}
                  />
                </label>
                <label>
                  {t("web.schedules.command_timeout", "超时毫秒")}
                  <input
                    value={commandTimeoutMs}
                    inputMode="numeric"
                    onChange={(event) => setCommandTimeoutMs(event.target.value)}
                  />
                </label>
              </div>
            </div>
          )}

          <div className="button-row">
            <button type="submit" disabled={busy || (mode === "ai" && actionOptions.length === 0)}>
              {t("web.schedules.create", "创建定时器")}
            </button>
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
            schedules.map((schedule) => (
              <article key={schedule.id} className="resource-card">
                <header>
                  <strong>{scheduleTitle(schedule)}</strong>
                  <span>
                    {schedule.enabled
                      ? t("web.common.enabled", "启用")
                      : t("web.common.disabled", "停用")}
                  </span>
                </header>
                <p>{schedule.id}</p>
                <div className="kv-list">
                  <span>{t("web.schedules.target", "目标")}</span>
                  <strong>{scheduleTargetLabel(schedule)}</strong>
                  <span>{t("web.schedules.trigger", "触发器")}</span>
                  <strong>{describeTrigger(schedule.trigger)}</strong>
                  <span>{t("web.schedules.next_run", "下次触发")}</span>
                  <strong>{formatScheduleTime(schedule.next_run_at)}</strong>
                  <span>{t("web.schedules.last_run", "上次触发")}</span>
                  <strong>{formatScheduleTime(schedule.last_run_at)}</strong>
                  <span>{t("web.common.status", "状态")}</span>
                  <strong>{schedule.last_status ?? "—"}</strong>
                  <span>{t("web.schedules.last_error", "最近错误")}</span>
                  <strong>{schedule.last_error ?? "—"}</strong>
                </div>
                <div className="button-row">
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
                        () => schedule.enabled ? pauseSchedule(schedule.id) : resumeSchedule(schedule.id),
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
            ))
          )}
        </div>
      </section>
    </div>
  );
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

function scheduleTitle(schedule: ScheduleRecord) {
  if (schedule.target.kind === "command") {
    return schedule.target.command.command;
  }
  return schedule.target.action_id;
}

function scheduleTargetLabel(schedule: ScheduleRecord) {
  if (schedule.target.kind === "command") {
    return schedule.target.command.cwd
      ? `command · ${schedule.target.command.cwd}`
      : "command";
  }
  return schedule.target.extension_id;
}
