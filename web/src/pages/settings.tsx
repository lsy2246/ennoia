import { useEffect, useState, type FormEvent } from "react";

import {
  fetchServerConfig,
  saveRuntimeProfile,
  saveServerConfig,
  type ServerConfig,
} from "@ennoia/api-client";
import { buildTimeZoneOptionGroups } from "@/lib/timeZones";
import { resolveDefaultDisplayName, resolveDefaultTimeZone } from "@/lib/uiDefaults";
import { Select } from "@/components/Select";
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
    <div className="stack settings-editor">
      <div className="settings-editor__header">
        <div className="panel-title">{title}</div>
        {helper ? <p className="helper-text">{helper}</p> : null}
      </div>
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
    <div className="stack settings-editor">
      <div className="settings-editor__header">
        <div className="panel-title">{title}</div>
        {helper ? <p className="helper-text">{helper}</p> : null}
      </div>
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
  const { runtime, t } = useUiHelpers();
  const defaultProfileName = resolveDefaultDisplayName(runtime);
  const defaultTimeZone = resolveDefaultTimeZone(runtime);
  const [config, setConfig] = useState<ServerConfig | null>(null);
  const [profileName, setProfileName] = useState(profile?.display_name ?? defaultProfileName);
  const [timeZone, setTimeZone] = useState(profile?.time_zone ?? defaultTimeZone);
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
    setProfileName(profile?.display_name ?? defaultProfileName);
    setTimeZone(profile?.time_zone ?? defaultTimeZone);
  }, [defaultProfileName, defaultTimeZone, profile]);

  async function hydrate() {
    const snapshot = await fetchServerConfig();
    setConfig(snapshot);
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
    if (!config) {
      return;
    }
    setError(null);
    setMessage(null);
    try {
      await saveServerConfig({
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
      });
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
    <div className="settings-page">
      <section className="work-panel settings-toolbar">
        <div className="settings-toolbar__row">
          <div className="settings-toolbar__copy">
            <span className="settings-panel__eyebrow">{t("web.settings.page_eyebrow", "系统设置")}</span>
            <h1>{t("web.settings.page_title_compact", "设置")}</h1>
            <p>
              {t(
                "web.settings.page_description_compact",
                "常用配置按小模块排开，桌面端更利于扫视，移动端保持单列编辑。",
              )}
            </p>
          </div>
          <div className="settings-toolbar__actions">
            <button
              type="button"
              onClick={() => void saveRuntimeConfig()}
              disabled={!config}
            >
              {t("web.settings.save_system", "保存系统设置")}
            </button>
          </div>
        </div>
      </section>

      {error ? <div className="error">{error}</div> : null}
      {message ? <div className="success">{message}</div> : null}

      <div className="settings-modular-grid">
        <form
          id="settings-personal"
          className="mini-card editor-form settings-section-card settings-panel settings-panel--compact settings-module"
          onSubmit={saveProfile}
        >
          <div className="settings-panel__header">
            <span className="settings-panel__eyebrow">{t("web.settings.personal", "个人设置")}</span>
            <div>
              <div className="panel-title">{t("web.settings.personal_title", "身份与时区")}</div>
              <p className="helper-text">
                {t("web.settings.personal_description", "决定工作台中的显示名称，以及所有时间的解释方式。")}
              </p>
            </div>
          </div>
          <div className="form-grid settings-form-grid">
            <label>
              {t("web.settings.operator_name", "操作者名称")}
              <input value={profileName} onChange={(event) => setProfileName(event.target.value)} />
            </label>
            <label>
              {t("web.settings.time_zone", "时区")}
              <Select
                value={timeZone}
                onChange={setTimeZone}
                options={buildTimeZoneOptionGroups(t, false).flatMap((group) =>
                  group.options.map((option) => ({
                    value: option.value,
                    label: option.label,
                    group: group.label,
                  }))
                )}
              />
            </label>
          </div>
          <div className="settings-actions settings-actions--inline">
            <button type="submit">{t("web.settings.save_personal", "保存个人设置")}</button>
          </div>
        </form>

        {config ? (
          <>
            <article
              id="settings-service"
              className="mini-card settings-section-card settings-section-anchor settings-module"
            >
                  <div className="settings-section-card__header">
                    <div className="panel-title">{t("web.settings.system_service_title", "服务基础")}</div>
                    <p className="helper-text">
                      {t("web.settings.system_service_help", "维护服务入口和基础运行行为。")}
                    </p>
                  </div>
                  <div className="form-grid settings-form-grid">
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
                      {t("web.settings.web_dev_host", "前端开发主机")}
                      <input
                        value={config.web_dev.host}
                        onChange={(event) =>
                          setConfig({
                            ...config,
                            web_dev: { ...config.web_dev, host: event.target.value },
                          })
                        }
                      />
                    </label>
                    <label>
                      {t("web.settings.web_dev_port", "前端开发端口")}
                      <input
                        value={config.web_dev.port}
                        onChange={(event) =>
                          setConfig({
                            ...config,
                            web_dev: { ...config.web_dev, port: Number(event.target.value) },
                          })
                        }
                      />
                    </label>
                  </div>
            </article>

            <section
              id="settings-providers"
              className="settings-provider-shell settings-module settings-section-anchor"
            >
              <Providers embedded />
            </section>

            <article
              id="settings-rate-limit"
              className="mini-card settings-section-card settings-section-anchor settings-module"
            >
                  <div className="settings-section-card__header">
                    <div className="panel-title">{t("web.settings.rate_limit", "限流")}</div>
                    <p className="helper-text">
                      {t("web.settings.rate_limit_help", "控制每个 IP 和用户在单位时间内的请求额度。")}
                    </p>
                  </div>
                  <label className="check-row settings-toggle-row">
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
                  <div className="form-grid settings-form-grid">
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
                    <label className="settings-field settings-field--wide">
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
            </article>

            <article
              id="settings-cors"
              className="mini-card settings-section-card settings-section-anchor settings-module"
            >
                  <div className="settings-section-card__header">
                    <div className="panel-title">{t("web.settings.cors", "跨域")}</div>
                    <p className="helper-text">
                      {t("web.settings.cors_help", "设置跨域访问和浏览器凭证的允许范围。")}
                    </p>
                  </div>
                  <div className="settings-toggle-group">
                    <label className="check-row settings-toggle-row">
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
                    <label className="check-row settings-toggle-row">
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
                  </div>
                  <StringListEditor
                    title={t("web.settings.origins", "允许来源")}
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
            </article>

            <article
              id="settings-timeout"
              className="mini-card settings-section-card settings-section-anchor settings-module"
            >
                  <div className="settings-section-card__header">
                    <div className="panel-title">{t("web.settings.timeout", "超时")}</div>
                    <p className="helper-text">
                      {t("web.settings.timeout_help", "为不同路径设置默认超时和定制覆盖值。")}
                    </p>
                  </div>
                  <label className="check-row settings-toggle-row">
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
                  <label className="settings-field">
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
            </article>

            <article
              id="settings-logging"
              className="mini-card settings-section-card settings-section-anchor settings-module"
            >
                  <div className="settings-section-card__header">
                    <div className="panel-title">{t("web.settings.logging", "日志")}</div>
                    <p className="helper-text">
                      {t("web.settings.logging_help", "控制日志等级、采样策略和开发态控制台镜像。")}
                    </p>
                  </div>
                  <label className="check-row settings-toggle-row">
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
                  <div className="form-grid settings-form-grid">
                    <label>
                      {t("web.settings.level", "级别")}
                      <Select
                        value={config.logging.level}
                        onChange={(value) =>
                          setConfig({
                            ...config,
                            logging: { ...config.logging, level: value },
                          })
                        }
                        options={[
                          { value: "debug", label: "debug" },
                          { value: "info", label: "info" },
                          { value: "warn", label: "warn" },
                          { value: "error", label: "error" },
                        ]}
                      />
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
                    <label className="check-row settings-toggle-row settings-field settings-field--wide">
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
                    <label className="settings-field settings-field--wide">
                      {t("settings.runtime.dev_console.level", "开发模式控制台级别")}
                      <Select
                        value={config.logging.dev_console.level}
                        onChange={(value) =>
                          setConfig({
                            ...config,
                            logging: {
                              ...config.logging,
                              dev_console: {
                                ...config.logging.dev_console,
                                level: value,
                              },
                            },
                          })
                        }
                        options={[
                          { value: "debug", label: "debug" },
                          { value: "info", label: "info" },
                          { value: "warn", label: "warn" },
                          { value: "error", label: "error" },
                        ]}
                      />
                    </label>
                  </div>
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
            </article>

            <article
              id="settings-body-limit"
              className="mini-card settings-section-card settings-section-anchor settings-module"
            >
                  <div className="settings-section-card__header">
                    <div className="panel-title">{t("web.settings.body_limit", "请求体限制")}</div>
                    <p className="helper-text">
                      {t("web.settings.body_limit_help", "按默认值和路径覆盖控制请求体大小。")}
                    </p>
                  </div>
                  <label className="check-row settings-toggle-row">
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
                  <label className="settings-field">
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
            </article>
          </>
        ) : (
          <div className="empty-card settings-loading-card">{t("web.common.loading", "加载中…")}</div>
        )}
      </div>
    </div>
  );
}
