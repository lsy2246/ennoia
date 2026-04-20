import { useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { getSchedule, updateSchedule, type ScheduleDetail } from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function ScheduleDetailPage() {
  const { scheduleId } = useParams({ from: "/shell/schedules/$scheduleId" });
  const { t, formatDateTime } = useUiHelpers();
  const [detail, setDetail] = useState<ScheduleDetail | null>(null);
  const [payloadJson, setPayloadJson] = useState("{}");
  const [scheduleValue, setScheduleValue] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const next = await getSchedule(scheduleId);
        if (!cancelled) {
          setDetail(next);
          setPayloadJson(next.payload_json);
          setScheduleValue(next.schedule_value);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [scheduleId]);

  async function handleSave() {
    if (!detail) {
      return;
    }
    try {
      const parsed = JSON.parse(payloadJson);
      const next = await updateSchedule(scheduleId, {
        schedule_value: scheduleValue,
        payload: parsed,
      });
      setDetail(next);
    } catch (err) {
      setError(String(err));
    }
  }

  if (error) {
    return <div className="page error">{error}</div>;
  }

  if (!detail) {
    return <div className="page">{t("shell.action.loading", "加载中…")}</div>;
  }

  return (
    <div className="page">
      <div className="surface-grid">
        <section className="surface-panel">
          <h1>{detail.job_kind}</h1>
          <dl className="data-pairs">
            <dt>ID</dt>
            <dd>{detail.id}</dd>
            <dt>{t("shell.common.status", "状态")}</dt>
            <dd>{detail.status}</dd>
            <dt>{t("shell.schedules.updated_at", "更新时间")}</dt>
            <dd>{formatDateTime(detail.updated_at)}</dd>
            <dt>{t("shell.schedules.next_run", "下次执行")}</dt>
            <dd>{detail.next_run_at ? formatDateTime(detail.next_run_at) : t("shell.common.none_plain", "无")}</dd>
          </dl>
        </section>

        <section className="surface-panel">
          <h2>{t("shell.schedules.edit", "编辑任务")}</h2>
          <div className="form-stack">
            <label>
              {t("shell.schedules.schedule_value", "调度值")}
              <input value={scheduleValue} onChange={(event) => setScheduleValue(event.target.value)} />
            </label>
            <label>
              Payload JSON
              <textarea rows={12} value={payloadJson} onChange={(event) => setPayloadJson(event.target.value)} />
            </label>
            <button onClick={() => void handleSave()}>{t("shell.action.save", "保存")}</button>
          </div>
        </section>
      </div>
    </div>
  );
}
