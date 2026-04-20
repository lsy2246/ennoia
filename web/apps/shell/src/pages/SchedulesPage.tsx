import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import {
  createSchedule,
  deleteSchedule,
  disableSchedule,
  enableSchedule,
  listSchedules,
  runScheduleNow,
  type Schedule,
} from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useUiHelpers } from "@/stores/ui";

export function SchedulesPage() {
  const navigate = useNavigate();
  const { t, formatDateTime } = useUiHelpers();
  const [items, setItems] = useState<Schedule[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [jobKind, setJobKind] = useState("maintenance");
  const [scheduleKind, setScheduleKind] = useState("once");
  const [scheduleValue, setScheduleValue] = useState("manual");

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      setItems(await listSchedules());
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function handleCreate() {
    try {
      const created = await createSchedule({
        owner_kind: "global",
        owner_id: "workspace",
        job_kind: jobKind,
        schedule_kind: scheduleKind,
        schedule_value: scheduleValue,
        payload: {},
      });
      await navigate({ to: "/schedules/$scheduleId", params: { scheduleId: created.id } });
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.schedules.title", "计划任务")}
        description={t(
          "shell.schedules.description",
          "统一管理长期计划任务。支持新建、编辑、删除、启停和立即执行，不再只是单向新增。",
        )}
      />

      {error ? <div className="error">{error}</div> : null}

      <div className="surface-grid">
        <section className="surface-panel">
          <h2>{t("shell.schedules.create", "新建任务")}</h2>
          <div className="form-stack">
            <label>
              {t("shell.schedules.kind", "任务类型")}
              <input value={jobKind} onChange={(event) => setJobKind(event.target.value)} />
            </label>
            <label>
              {t("shell.schedules.schedule_kind", "调度方式")}
              <select value={scheduleKind} onChange={(event) => setScheduleKind(event.target.value)}>
                <option value="once">once</option>
                <option value="delay">delay</option>
                <option value="interval">interval</option>
                <option value="cron">cron</option>
              </select>
            </label>
            <label>
              {t("shell.schedules.schedule_value", "调度值")}
              <input value={scheduleValue} onChange={(event) => setScheduleValue(event.target.value)} />
            </label>
            <button onClick={() => void handleCreate()}>{t("shell.schedules.create_action", "创建任务")}</button>
          </div>
        </section>

        <section className="surface-panel">
          <div className="section-heading">
            <h2>{t("shell.schedules.list", "任务列表")}</h2>
            <button className="secondary" onClick={() => void refresh()}>
              {t("shell.action.refresh", "刷新")}
            </button>
          </div>

          {loading ? <p className="muted">{t("shell.action.loading", "加载中…")}</p> : null}

          <div className="stack-list">
            {items.map((item) => (
              <article key={item.id} className="thread-card">
                <div>
                  <div className="thread-card__title">
                    <Link to="/schedules/$scheduleId" params={{ scheduleId: item.id }}>
                      {item.job_kind}
                    </Link>
                  </div>
                  <p>
                    {item.schedule_kind} · {item.schedule_value}
                  </p>
                  <span>
                    {item.status} · {formatDateTime(item.created_at)}
                  </span>
                </div>
                <div className="button-row">
                  <button className="secondary" onClick={() => void runScheduleNow(item.id).then(refresh)}>
                    {t("shell.schedules.run_now", "立即执行")}
                  </button>
                  {item.status === "disabled" ? (
                    <button className="secondary" onClick={() => void enableSchedule(item.id).then(refresh)}>
                      {t("shell.schedules.enable", "启用")}
                    </button>
                  ) : (
                    <button className="secondary" onClick={() => void disableSchedule(item.id).then(refresh)}>
                      {t("shell.schedules.disable", "停用")}
                    </button>
                  )}
                  <button className="danger" onClick={() => void deleteSchedule(item.id).then(refresh)}>
                    {t("shell.action.delete", "删除")}
                  </button>
                </div>
              </article>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
