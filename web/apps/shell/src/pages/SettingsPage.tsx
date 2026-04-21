import { useEffect, useState, type FormEvent } from "react";

import {
  fetchAppConfig,
  getConfigSnapshot,
  putConfig,
  saveAppConfig,
  saveRuntimeProfile,
  type AppConfig,
  type SystemConfig,
} from "@ennoia/api-client";
import { WORKBENCH_PALETTES, applyWorkbenchPalette, readWorkbenchPalette } from "@/lib/palette";
import { buildTimeZoneOptionGroups } from "@/lib/timeZones";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

function splitText(value: string) {
  return value
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function SettingsPage() {
  const profile = useRuntimeStore((state) => state.profile);
  const hydrateRuntime = useRuntimeStore((state) => state.hydrate);
  const uiState = useUiStore();
  const { availableLocales, availableThemes, t } = useUiHelpers();
  const [config, setConfig] = useState<SystemConfig | null>(null);
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  const [profileName, setProfileName] = useState(profile?.display_name ?? "Operator");
  const [timeZone, setTimeZone] = useState(profile?.time_zone ?? "Asia/Shanghai");
  const [palette, setPalette] = useState(readWorkbenchPalette);
  const [corsOrigins, setCorsOrigins] = useState("");
  const [timeoutOverrides, setTimeoutOverrides] = useState("");
  const [bodyLimitOverrides, setBodyLimitOverrides] = useState("");
  const [redactHeaders, setRedactHeaders] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    void hydrate();
  }, []);

  useEffect(() => {
    if (profile) {
      setProfileName(profile.display_name);
      setTimeZone(profile.time_zone);
    }
  }, [profile]);

  async function hydrate() {
    const [snapshot, nextAppConfig] = await Promise.all([getConfigSnapshot(), fetchAppConfig()]);
    setConfig(snapshot);
    setAppConfig(nextAppConfig);
    setCorsOrigins(snapshot.cors.origins.join("\n"));
    setTimeoutOverrides(
      Object.entries(snapshot.timeout.per_path_ms)
        .map(([key, value]) => `${key}=${value}`)
        .join("\n"),
    );
    setBodyLimitOverrides(
      Object.entries(snapshot.body_limit.per_path_max)
        .map(([key, value]) => `${key}=${value}`)
        .join("\n"),
    );
    setRedactHeaders(snapshot.logging.redact_headers.join(", "));
  }

  function parseMap(text: string) {
    return Object.fromEntries(
      text
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter(Boolean)
        .map((line) => {
          const [key, value] = line.split("=");
          return [key.trim(), Number(value)];
        }),
    );
  }

  async function saveProfile(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setMessage(null);
    try {
      await saveRuntimeProfile({
        display_name: profileName,
        time_zone: timeZone,
      });
      await hydrateRuntime();
      setMessage(t("web.settings.profile_saved", "个人设置已保存。"));
    } catch (err) {
      setError(String(err));
    }
  }

  async function savePreferences(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setMessage(null);
    try {
      await uiState.savePreferences({
        locale: uiState.locale,
        theme_id: uiState.themeId,
        time_zone: timeZone,
      });
      setMessage(t("web.settings.preferences_saved", "界面偏好已保存。"));
    } catch (err) {
      setError(String(err));
    }
  }

  async function changeLocale(locale: string) {
    setError(null);
    setMessage(null);
    try {
      await uiState.savePreferences({
        locale,
        theme_id: uiState.themeId,
        time_zone: timeZone,
      });
    } catch (err) {
      setError(String(err));
    }
  }

  async function changeTheme(themeId: string) {
    setError(null);
    setMessage(null);
    try {
      await uiState.savePreferences({
        locale: uiState.locale,
        theme_id: themeId,
        time_zone: timeZone,
      });
    } catch (err) {
      setError(String(err));
    }
  }

  async function saveRuntimeConfig() {
    if (!config || !appConfig) {
      return;
    }
    setError(null);
    setMessage(null);
    try {
      await Promise.all([
        saveAppConfig(appConfig),
        putConfig("rate_limit", config.rate_limit, "shell"),
        putConfig(
          "cors",
          {
            ...config.cors,
            origins: splitText(corsOrigins),
          },
          "shell",
        ),
        putConfig(
          "timeout",
          {
            ...config.timeout,
            per_path_ms: parseMap(timeoutOverrides),
          },
          "shell",
        ),
        putConfig(
          "logging",
          {
            ...config.logging,
            redact_headers: splitText(redactHeaders),
          },
          "shell",
        ),
        putConfig(
          "body_limit",
          {
            ...config.body_limit,
            per_path_max: parseMap(bodyLimitOverrides),
          },
          "shell",
        ),
      ]);
      await hydrate();
      setMessage(t("web.settings.runtime_saved", "运行时配置已保存。"));
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="settings-grid">
      <form className="work-panel editor-form" onSubmit={saveProfile}>
        <div className="panel-title">{t("web.settings.personal", "个人设置")}</div>
        <label>{t("web.settings.operator_name", "操作者名称")}<input value={profileName} onChange={(event) => setProfileName(event.target.value)} /></label>
        <label>
          {t("web.settings.time_zone", "时区")}
          <select value={timeZone} onChange={(event) => setTimeZone(event.target.value)}>
            {buildTimeZoneOptionGroups(t, false).map((group) => (
              <optgroup key={group.label} label={group.label}>
                {group.options.map((option) => (
                  <option key={option.value} value={option.value}>{option.label}</option>
                ))}
              </optgroup>
            ))}
          </select>
        </label>
        <button type="submit">{t("web.settings.save_personal", "保存个人设置")}</button>
      </form>

      <form className="work-panel editor-form" onSubmit={savePreferences}>
        <div className="panel-title">{t("web.settings.appearance", "外观与配色")}</div>
        <label>
          {t("web.settings.language", "语言")}
          <select value={uiState.locale} onChange={(event) => void changeLocale(event.target.value)}>
            {availableLocales.map((locale) => (
              <option key={locale} value={locale}>{locale}</option>
            ))}
          </select>
        </label>
        <label>
          {t("web.settings.theme", "主题")}
          <select value={uiState.themeId} onChange={(event) => void changeTheme(event.target.value)}>
            {availableThemes.map((theme) => (
              <option key={theme.id} value={theme.id}>{theme.label}</option>
            ))}
          </select>
        </label>
        <div className="palette-grid">
          {WORKBENCH_PALETTES.map((item) => (
            <button
              key={item.id}
              type="button"
              className={palette === item.id ? "palette-card palette-card--active" : "palette-card"}
              onClick={() => {
                setPalette(applyWorkbenchPalette(item.id));
              }}
            >
              <strong>{t(`web.palette.${item.id}.label`, item.label)}</strong>
              <span>{t(`web.palette.${item.id}.description`, item.description)}</span>
            </button>
          ))}
        </div>
        <button type="submit">{t("web.settings.save_appearance", "保存外观设置")}</button>
      </form>

      <section className="work-panel editor-form settings-wide">
        <div className="page-heading">
          <span>{t("web.settings.runtime_eyebrow", "Runtime Config")}</span>
          <h1>{t("web.settings.runtime_title", "运行时配置改成表单，不再暴露 JSON 编辑器。")}</h1>
          <p>{t("web.settings.runtime_description", "覆盖工作区、API 上游渠道扫描、调度节拍、中间件开关、跨域、超时、日志脱敏与请求体大小。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        {message ? <div className="success">{message}</div> : null}
        {config && appConfig ? (
          <>
            <div className="config-sections">
              <div className="mini-card">
                <div className="panel-title">{t("web.settings.app_config", "应用与工作区")}</div>
                <label>{t("web.settings.workspace_root", "工作区根路径")}<input value={appConfig.workspace_root} onChange={(event) => setAppConfig({ ...appConfig, workspace_root: event.target.value })} /><p className="helper-text">{t("web.settings.workspace_root_help", "唯一工作区入口。Session 工作区、产物和临时目录都基于它派生。")}</p></label>
                <div className="form-grid">
                  <label>{t("web.settings.extensions_scan", "扩展扫描目录")}<input value={appConfig.extensions_scan_dir} onChange={(event) => setAppConfig({ ...appConfig, extensions_scan_dir: event.target.value })} /></label>
                  <label>{t("web.settings.agents_scan", "Agent 配置目录")}<input value={appConfig.agents_scan_dir} onChange={(event) => setAppConfig({ ...appConfig, agents_scan_dir: event.target.value })} /></label>
                  <label>{t("web.settings.scheduler_tick", "调度器轮询毫秒")}<input value={appConfig.scheduler_tick_ms} onChange={(event) => setAppConfig({ ...appConfig, scheduler_tick_ms: Number(event.target.value) })} /></label>
                  <label>{t("web.settings.default_mentions", "默认提及策略")}<input value={appConfig.default_mention_mode} onChange={(event) => setAppConfig({ ...appConfig, default_mention_mode: event.target.value })} /></label>
                </div>
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.rate_limit", "Rate Limit")}</div>
                <label className="check-row"><input type="checkbox" checked={config.rate_limit.enabled} onChange={(event) => setConfig({ ...config, rate_limit: { ...config.rate_limit, enabled: event.target.checked } })} />{t("web.common.enabled", "启用")}</label>
                <div className="form-grid">
                  <label>IP RPM<input value={config.rate_limit.per_ip_rpm} onChange={(event) => setConfig({ ...config, rate_limit: { ...config.rate_limit, per_ip_rpm: Number(event.target.value) } })} /></label>
                  <label>User RPM<input value={config.rate_limit.per_user_rpm} onChange={(event) => setConfig({ ...config, rate_limit: { ...config.rate_limit, per_user_rpm: Number(event.target.value) } })} /></label>
                  <label>Burst<input value={config.rate_limit.burst} onChange={(event) => setConfig({ ...config, rate_limit: { ...config.rate_limit, burst: Number(event.target.value) } })} /></label>
                </div>
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.cors", "CORS")}</div>
                <label className="check-row"><input type="checkbox" checked={config.cors.enabled} onChange={(event) => setConfig({ ...config, cors: { ...config.cors, enabled: event.target.checked } })} />{t("web.common.enabled", "启用")}</label>
                <label className="check-row"><input type="checkbox" checked={config.cors.credentials} onChange={(event) => setConfig({ ...config, cors: { ...config.cors, credentials: event.target.checked } })} />允许凭证</label>
                <label>{t("web.settings.origins", "Origins")}<textarea rows={5} value={corsOrigins} onChange={(event) => setCorsOrigins(event.target.value)} /></label>
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.timeout", "Timeout")}</div>
                <label className="check-row"><input type="checkbox" checked={config.timeout.enabled} onChange={(event) => setConfig({ ...config, timeout: { ...config.timeout, enabled: event.target.checked } })} />{t("web.common.enabled", "启用")}</label>
                <label>{t("web.settings.default_ms", "默认毫秒")}<input value={config.timeout.default_ms} onChange={(event) => setConfig({ ...config, timeout: { ...config.timeout, default_ms: Number(event.target.value) } })} /></label>
                <label>{t("web.settings.path_ms", "路径覆盖（/path=ms）")}<textarea rows={5} value={timeoutOverrides} onChange={(event) => setTimeoutOverrides(event.target.value)} /></label>
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.logging", "Logging")}</div>
                <label className="check-row"><input type="checkbox" checked={config.logging.enabled} onChange={(event) => setConfig({ ...config, logging: { ...config.logging, enabled: event.target.checked } })} />{t("web.common.enabled", "启用")}</label>
                <label>{t("web.settings.level", "级别")}<select value={config.logging.level} onChange={(event) => setConfig({ ...config, logging: { ...config.logging, level: event.target.value } })}><option value="debug">debug</option><option value="info">info</option><option value="warn">warn</option><option value="error">error</option></select></label>
                <label>{t("web.settings.sample_rate", "采样率")}<input value={config.logging.sample_rate} onChange={(event) => setConfig({ ...config, logging: { ...config.logging, sample_rate: Number(event.target.value) } })} /></label>
                <label>{t("web.settings.redact_headers", "脱敏请求头")}<textarea rows={4} value={redactHeaders} onChange={(event) => setRedactHeaders(event.target.value)} /></label>
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.body_limit", "Body Limit")}</div>
                <label className="check-row"><input type="checkbox" checked={config.body_limit.enabled} onChange={(event) => setConfig({ ...config, body_limit: { ...config.body_limit, enabled: event.target.checked } })} />{t("web.common.enabled", "启用")}</label>
                <label>{t("web.settings.default_bytes", "默认字节数")}<input value={config.body_limit.max_bytes} onChange={(event) => setConfig({ ...config, body_limit: { ...config.body_limit, max_bytes: Number(event.target.value) } })} /></label>
                <label>{t("web.settings.path_bytes", "路径覆盖（/path=bytes）")}<textarea rows={5} value={bodyLimitOverrides} onChange={(event) => setBodyLimitOverrides(event.target.value)} /></label>
              </div>
            </div>
            <button type="button" onClick={() => void saveRuntimeConfig()}>{t("web.settings.save_runtime", "保存运行时配置")}</button>
          </>
        ) : (
          <div className="empty-card">{t("web.common.loading", "加载中…")}</div>
        )}
      </section>
    </div>
  );
}
