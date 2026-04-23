import { useEffect, useState, type FormEvent } from "react";

import {
  fetchAppConfig,
  fetchServerConfig,
  saveAppConfig,
  saveRuntimeProfile,
  saveServerConfig,
  type AppConfig,
  type ServerConfig,
} from "@ennoia/api-client";
import { buildTimeZoneOptionGroups } from "@/lib/timeZones";
import { Providers } from "@/pages/providers";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers } from "@/stores/ui";

type StringEntry = {
  key: string;
  value: string;
};

type NumberMapEntry = {
  key: string;
  path: string;
  value: string;
};

let entrySequence = 0;

function createStringEntry(value = ""): StringEntry {
  entrySequence += 1;
  return { key: `string-entry-${entrySequence}`, value };
}

function createNumberMapEntry(path = "", value = ""): NumberMapEntry {
  entrySequence += 1;
  return { key: `map-entry-${entrySequence}`, path, value };
}

function normalizeTextList(values: string[]) {
  return Array.from(new Set(values.map((item) => item.trim()).filter(Boolean)));
}

function toStringEntries(values: string[]) {
  return normalizeTextList(values).map((item) => createStringEntry(item));
}

function toMapEntries(values: Record<string, number>) {
  return Object.entries(values).map(([path, value]) => createNumberMapEntry(path, String(value)));
}

function collectStringEntries(entries: StringEntry[]) {
  return normalizeTextList(entries.map((entry) => entry.value));
}

function collectMapEntries(entries: NumberMapEntry[]) {
  return Object.fromEntries(
    entries
      .map((entry) => ({
        path: entry.path.trim(),
        value: Number(entry.value),
      }))
      .filter((entry) => entry.path && Number.isFinite(entry.value))
      .map((entry) => [entry.path, entry.value] as const),
  );
}

function StringListEditor({
  title,
  helper,
  entries,
  emptyText,
  placeholder,
  addLabel,
  deleteLabel,
  onChange,
}: {
  title: string;
  helper?: string;
  entries: StringEntry[];
  emptyText: string;
  placeholder: string;
  addLabel: string;
  deleteLabel: string;
  onChange: (entries: StringEntry[]) => void;
}) {
  function updateValue(key: string, value: string) {
    onChange(entries.map((entry) => (entry.key === key ? { ...entry, value } : entry)));
  }

  function removeValue(key: string) {
    onChange(entries.filter((entry) => entry.key !== key));
  }

  function addValue() {
    onChange([...entries, createStringEntry()]);
  }

  return (
    <div className="stack">
      <div className="panel-title">{title}</div>
      {helper ? <p className="helper-text">{helper}</p> : null}
      <div className="editor-list">
        {entries.length === 0 ? (
          <div className="empty-card">{emptyText}</div>
        ) : (
          entries.map((entry) => (
            <div key={entry.key} className="editor-row">
              <input
                value={entry.value}
                placeholder={placeholder}
                onChange={(event) => updateValue(entry.key, event.target.value)}
              />
              <button type="button" className="secondary" onClick={() => removeValue(entry.key)}>
                {deleteLabel}
              </button>
            </div>
          ))
        )}
        <button type="button" className="secondary" onClick={addValue}>
          {addLabel}
        </button>
      </div>
    </div>
  );
}

function NumberMapEditor({
  title,
  helper,
  entries,
  emptyText,
  pathPlaceholder,
  valuePlaceholder,
  addLabel,
  deleteLabel,
  onChange,
}: {
  title: string;
  helper?: string;
  entries: NumberMapEntry[];
  emptyText: string;
  pathPlaceholder: string;
  valuePlaceholder: string;
  addLabel: string;
  deleteLabel: string;
  onChange: (entries: NumberMapEntry[]) => void;
}) {
  function updateEntry(key: string, patch: Partial<NumberMapEntry>) {
    onChange(entries.map((entry) => (entry.key === key ? { ...entry, ...patch } : entry)));
  }

  function removeEntry(key: string) {
    onChange(entries.filter((entry) => entry.key !== key));
  }

  function addEntry() {
    onChange([...entries, createNumberMapEntry()]);
  }

  return (
    <div className="stack">
      <div className="panel-title">{title}</div>
      {helper ? <p className="helper-text">{helper}</p> : null}
      <div className="editor-list">
        {entries.length === 0 ? (
          <div className="empty-card">{emptyText}</div>
        ) : (
          entries.map((entry) => (
            <div key={entry.key} className="editor-row editor-row--split">
              <input
                value={entry.path}
                placeholder={pathPlaceholder}
                onChange={(event) => updateEntry(entry.key, { path: event.target.value })}
              />
              <input
                value={entry.value}
                inputMode="numeric"
                placeholder={valuePlaceholder}
                onChange={(event) => updateEntry(entry.key, { value: event.target.value })}
              />
              <button type="button" className="secondary" onClick={() => removeEntry(entry.key)}>
                {deleteLabel}
              </button>
            </div>
          ))
        )}
        <button type="button" className="secondary" onClick={addEntry}>
          {addLabel}
        </button>
      </div>
    </div>
  );
}

export function Settings() {
  const profile = useRuntimeStore((state) => state.profile);
  const hydrateRuntime = useRuntimeStore((state) => state.hydrate);
  const { t } = useUiHelpers();
  const [config, setConfig] = useState<ServerConfig | null>(null);
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  const [profileName, setProfileName] = useState(profile?.display_name ?? "Operator");
  const [timeZone, setTimeZone] = useState(profile?.time_zone ?? "Asia/Shanghai");
  const [corsOrigins, setCorsOrigins] = useState<StringEntry[]>([]);
  const [timeoutOverrides, setTimeoutOverrides] = useState<NumberMapEntry[]>([]);
  const [bodyLimitOverrides, setBodyLimitOverrides] = useState<NumberMapEntry[]>([]);
  const [redactHeaders, setRedactHeaders] = useState<StringEntry[]>([]);
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
    const [snapshot, nextAppConfig] = await Promise.all([
      fetchServerConfig(),
      fetchAppConfig(),
    ]);
    setConfig(snapshot);
    setAppConfig(nextAppConfig);
    setCorsOrigins(toStringEntries(snapshot.cors.origins));
    setTimeoutOverrides(toMapEntries(snapshot.timeout.per_path_ms));
    setBodyLimitOverrides(toMapEntries(snapshot.body_limit.per_path_max));
    setRedactHeaders(toStringEntries(snapshot.logging.redact_headers));
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

  async function saveRuntimeConfig() {
    if (!config || !appConfig) {
      return;
    }
    setError(null);
    setMessage(null);
    try {
      await Promise.all([
        saveAppConfig(appConfig),
        saveServerConfig({
          ...config,
          cors: {
            ...config.cors,
            origins: collectStringEntries(corsOrigins),
          },
          timeout: {
            ...config.timeout,
            per_path_ms: collectMapEntries(timeoutOverrides),
          },
          logging: {
            ...config.logging,
            redact_headers: collectStringEntries(redactHeaders),
          },
          body_limit: {
            ...config.body_limit,
            per_path_max: collectMapEntries(bodyLimitOverrides),
          },
        }),
      ]);
      await hydrate();
      setMessage(t("web.settings.runtime_saved", "运行时配置已保存。"));
    } catch (err) {
      setError(String(err));
    }
  }

  const deleteLabel = t("web.action.delete", "删除");
  const addItemLabel = t("web.settings.list_add", "新增一项");
  const addRuleLabel = t("web.settings.map_add", "新增规则");

  return (
    <div className="settings-grid">
      <form className="work-panel editor-form" onSubmit={saveProfile}>
        <div className="panel-title">{t("web.settings.personal", "个人设置")}</div>
        <label>
          {t("web.settings.operator_name", "操作者名称")}
          <input value={profileName} onChange={(event) => setProfileName(event.target.value)} />
        </label>
        <label>
          {t("web.settings.time_zone", "时区")}
          <select value={timeZone} onChange={(event) => setTimeZone(event.target.value)}>
            {buildTimeZoneOptionGroups(t, false).map((group) => (
              <optgroup key={group.label} label={group.label}>
                {group.options.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        </label>
        <button type="submit">{t("web.settings.save_personal", "保存个人设置")}</button>
      </form>

      <section className="settings-wide">
        <Providers />
      </section>

      <section className="work-panel editor-form settings-wide">
        <div className="page-heading">
          <span>{t("web.settings.runtime_eyebrow", "Runtime Config")}</span>
          <h1>{t("web.settings.runtime_title", "运行时配置改成表单，不再暴露 JSON 编辑器。")}</h1>
          <p>
            {t(
              "web.settings.runtime_description",
              "覆盖系统配置文件、中间件开关、跨域、超时、日志脱敏与请求体大小。",
            )}
          </p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        {message ? <div className="success">{message}</div> : null}
        {config && appConfig ? (
          <>
            <div className="config-sections">
              <div className="mini-card">
                <div className="panel-title">{t("web.settings.app_config", "应用配置")}</div>
                <div className="form-grid">
                  <label>
                    {t("web.settings.server_host", "服务主机")}
                    <input
                      value={config.host}
                      onChange={(event) => setConfig({ ...config, host: event.target.value })}
                    />
                  </label>
                  <label>
                    {t("web.settings.server_port", "服务端口")}
                    <input
                      value={config.port}
                      onChange={(event) =>
                        setConfig({ ...config, port: Number(event.target.value) })
                      }
                    />
                  </label>
                  <label>
                    {t("web.settings.default_mentions", "默认提及策略")}
                    <input
                      value={appConfig.default_mention_mode}
                      onChange={(event) =>
                        setAppConfig({
                          ...appConfig,
                          default_mention_mode: event.target.value,
                        })
                      }
                    />
                  </label>
                </div>
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.rate_limit", "Rate Limit")}</div>
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={config.rate_limit.enabled}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        rate_limit: { ...config.rate_limit, enabled: event.target.checked },
                      })
                    }
                  />
                  {t("web.common.enabled", "启用")}
                </label>
                <div className="form-grid">
                  <label>
                    IP RPM
                    <input
                      value={config.rate_limit.per_ip_rpm}
                      onChange={(event) =>
                        setConfig({
                          ...config,
                          rate_limit: {
                            ...config.rate_limit,
                            per_ip_rpm: Number(event.target.value),
                          },
                        })
                      }
                    />
                  </label>
                  <label>
                    User RPM
                    <input
                      value={config.rate_limit.per_user_rpm}
                      onChange={(event) =>
                        setConfig({
                          ...config,
                          rate_limit: {
                            ...config.rate_limit,
                            per_user_rpm: Number(event.target.value),
                          },
                        })
                      }
                    />
                  </label>
                  <label>
                    Burst
                    <input
                      value={config.rate_limit.burst}
                      onChange={(event) =>
                        setConfig({
                          ...config,
                          rate_limit: { ...config.rate_limit, burst: Number(event.target.value) },
                        })
                      }
                    />
                  </label>
                </div>
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.cors", "CORS")}</div>
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={config.cors.enabled}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        cors: { ...config.cors, enabled: event.target.checked },
                      })
                    }
                  />
                  {t("web.common.enabled", "启用")}
                </label>
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={config.cors.credentials}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        cors: { ...config.cors, credentials: event.target.checked },
                      })
                    }
                  />
                  {t("web.settings.credentials", "允许凭证")}
                </label>
                <StringListEditor
                  title={t("web.settings.origins", "Origins")}
                  helper={t("web.settings.origins_help", "每项填写一个允许的来源地址。")}
                  entries={corsOrigins}
                  emptyText={t("web.settings.origins_empty", "还没有允许来源。")}
                  placeholder={t(
                    "web.settings.origin_placeholder",
                    "例如 http://127.0.0.1:5173",
                  )}
                  addLabel={addItemLabel}
                  deleteLabel={deleteLabel}
                  onChange={setCorsOrigins}
                />
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.timeout", "Timeout")}</div>
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={config.timeout.enabled}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        timeout: { ...config.timeout, enabled: event.target.checked },
                      })
                    }
                  />
                  {t("web.common.enabled", "启用")}
                </label>
                <label>
                  {t("web.settings.default_ms", "默认毫秒")}
                  <input
                    value={config.timeout.default_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        timeout: { ...config.timeout, default_ms: Number(event.target.value) },
                      })
                    }
                  />
                </label>
                <NumberMapEditor
                  title={t("web.settings.path_ms", "路径覆盖（/path=ms）")}
                  helper={t("web.settings.path_overrides_help", "逐项填写路径和超时毫秒值。")}
                  entries={timeoutOverrides}
                  emptyText={t("web.settings.path_overrides_empty", "还没有路径覆盖规则。")}
                  pathPlaceholder={t("web.settings.path_placeholder", "例如 /api/logs")}
                  valuePlaceholder={t("web.settings.ms_placeholder", "毫秒")}
                  addLabel={addRuleLabel}
                  deleteLabel={deleteLabel}
                  onChange={setTimeoutOverrides}
                />
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.logging", "Logging")}</div>
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={config.logging.enabled}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        logging: { ...config.logging, enabled: event.target.checked },
                      })
                    }
                  />
                  {t("web.common.enabled", "启用")}
                </label>
                <label>
                  {t("web.settings.level", "级别")}
                  <select
                    value={config.logging.level}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        logging: { ...config.logging, level: event.target.value },
                      })
                    }
                  >
                    <option value="debug">debug</option>
                    <option value="info">info</option>
                    <option value="warn">warn</option>
                    <option value="error">error</option>
                  </select>
                </label>
                <label>
                  {t("web.settings.sample_rate", "采样率")}
                  <input
                    value={config.logging.sample_rate}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        logging: {
                          ...config.logging,
                          sample_rate: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={config.logging.dev_console.enabled}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        logging: {
                          ...config.logging,
                          dev_console: {
                            ...config.logging.dev_console,
                            enabled: event.target.checked,
                          },
                        },
                      })
                    }
                  />
                  {t("settings.runtime.dev_console.enabled", "开发模式镜像日志到命令窗口")}
                </label>
                <label>
                  {t("settings.runtime.dev_console.level", "开发模式控制台级别")}
                  <select
                    value={config.logging.dev_console.level}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        logging: {
                          ...config.logging,
                          dev_console: {
                            ...config.logging.dev_console,
                            level: event.target.value,
                          },
                        },
                      })
                    }
                  >
                    <option value="debug">debug</option>
                    <option value="info">info</option>
                    <option value="warn">warn</option>
                    <option value="error">error</option>
                  </select>
                </label>
                <StringListEditor
                  title={t("web.settings.redact_headers", "脱敏请求头")}
                  helper={t(
                    "web.settings.redact_headers_help",
                    "每项填写一个需要脱敏的请求头名称。",
                  )}
                  entries={redactHeaders}
                  emptyText={t("web.settings.redact_headers_empty", "还没有脱敏请求头。")}
                  placeholder={t("web.settings.header_placeholder", "例如 authorization")}
                  addLabel={addItemLabel}
                  deleteLabel={deleteLabel}
                  onChange={setRedactHeaders}
                />
              </div>

              <div className="mini-card">
                <div className="panel-title">{t("web.settings.body_limit", "Body Limit")}</div>
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={config.body_limit.enabled}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        body_limit: { ...config.body_limit, enabled: event.target.checked },
                      })
                    }
                  />
                  {t("web.common.enabled", "启用")}
                </label>
                <label>
                  {t("web.settings.default_bytes", "默认字节数")}
                  <input
                    value={config.body_limit.max_bytes}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        body_limit: {
                          ...config.body_limit,
                          max_bytes: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <NumberMapEditor
                  title={t("web.settings.path_bytes", "路径覆盖（/path=bytes）")}
                  helper={t(
                    "web.settings.path_size_overrides_help",
                    "逐项填写路径和最大字节数。",
                  )}
                  entries={bodyLimitOverrides}
                  emptyText={t("web.settings.path_overrides_empty", "还没有路径覆盖规则。")}
                  pathPlaceholder={t("web.settings.path_placeholder", "例如 /api/messages")}
                  valuePlaceholder={t("web.settings.bytes_placeholder", "字节数")}
                  addLabel={addRuleLabel}
                  deleteLabel={deleteLabel}
                  onChange={setBodyLimitOverrides}
                />
              </div>
            </div>
            <button type="button" onClick={() => void saveRuntimeConfig()}>
              {t("web.settings.save_runtime", "保存运行时配置")}
            </button>
          </>
        ) : (
          <div className="empty-card">{t("web.common.loading", "加载中…")}</div>
        )}
      </section>
    </div>
  );
}
