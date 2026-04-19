import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState, type FormEvent } from "react";

import { bootstrapSetup } from "@ennoia/api-client";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

export function WelcomePage() {
  const navigate = useNavigate();
  const bootstrap = useRuntimeStore((state) => state.bootstrap);
  const hydrateRuntime = useRuntimeStore((state) => state.hydrate);
  const hydrateUi = useUiStore((state) => state.hydrate);
  const { availableThemes } = useUiHelpers();

  const [displayName, setDisplayName] = useState("Operator");
  const [locale, setLocale] = useState(
    typeof navigator !== "undefined" ? navigator.language : "zh-CN",
  );
  const [timeZone, setTimeZone] = useState(
    Intl.DateTimeFormat().resolvedOptions().timeZone || "Asia/Shanghai",
  );
  const [themeId, setThemeId] = useState("system");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (bootstrap?.is_initialized) {
      navigate({ to: "/conversations" });
    }
  }, [bootstrap, navigate]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await bootstrapSetup({
        display_name: displayName,
        locale,
        time_zone: timeZone,
        theme_id: themeId,
      });
      await Promise.all([hydrateRuntime(), hydrateUi()]);
      navigate({ to: "/conversations" });
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page page--centered onboarding-page">
      <section className="onboarding-hero">
        <span className="onboarding-hero__eyebrow">首次启动引导</span>
        <h1>先确定这一套工作台的基础偏好，再开始协作。</h1>
        <p className="onboarding-hero__lead">
          Ennoia 现在是单操作者、多 Agent 的本地工作台。这里不需要账号，初始化完成后会直接进入工作区。
        </p>
      </section>

      <form onSubmit={handleSubmit} className="setup-card setup-card--wide onboarding-card">
        <div className="onboarding-card__header">
          <h2>工作台初始化</h2>
          <p>这些信息会写入当前实例，并作为浏览器缓存与服务端偏好的初始值。</p>
        </div>

        <div className="form-stack">
          <div className="form-row">
            <label>
              操作者名称
              <input value={displayName} onChange={(event) => setDisplayName(event.target.value)} />
            </label>
            <label>
              语言
              <input value={locale} onChange={(event) => setLocale(event.target.value)} />
            </label>
          </div>

          <div className="form-row">
            <label>
              时区
              <input value={timeZone} onChange={(event) => setTimeZone(event.target.value)} />
            </label>
            <label>
              主题
              <select value={themeId} onChange={(event) => setThemeId(event.target.value)}>
                {availableThemes.map((theme) => (
                  <option key={theme.id} value={theme.id}>
                    {theme.label}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </div>

        {error && <div className="setup-card__error">{error}</div>}

        <div className="onboarding-actions">
          <button type="submit" disabled={busy}>
            {busy ? "正在初始化…" : "完成初始化并进入工作台"}
          </button>
        </div>
      </form>
    </div>
  );
}
