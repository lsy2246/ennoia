import { useEffect, useState, type FormEvent } from "react";

import {
  fetchServerConfig,
  fetchUiConfig,
  saveRuntimeProfile,
  saveServerConfig,
  type ServerConfig,
  saveUiConfig,
  type UiConfig,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { buildTimeZoneOptionGroups } from "@/lib/timeZones";
import { resolveDefaultDisplayName, resolveDefaultTimeZone } from "@/lib/uiDefaults";
import { Select } from "@/components/Select";
import { ModelEndpointsPage } from "@/pages/model-endpoints";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

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
  const refreshUiRuntime = useUiStore((state) => state.refreshRuntime);
  const { runtime, t } = useUiHelpers();
  const defaultProfileName = resolveDefaultDisplayName(runtime);
  const defaultTimeZone = resolveDefaultTimeZone(runtime);
  const [config, setConfig] = useState<ServerConfig | null>(null);
  const [uiConfig, setUiConfig] = useState<UiConfig | null>(null);
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
    const [serverSnapshot, uiSnapshot] = await Promise.all([
      fetchServerConfig(),
      fetchUiConfig(),
    ]);
    setConfig(serverSnapshot);
    setUiConfig(uiSnapshot);
    setCorsOrigins(toStringEntries(serverSnapshot.cors.origins));
    setTimeoutOverrides(toMapEntries(serverSnapshot.timeout.per_path_ms));
    setBodyLimitOverrides(toMapEntries(serverSnapshot.body_limit.per_path_max));
    setRedactHeaders(toStringEntries(serverSnapshot.logging.redact_headers));
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
    if (!config || !uiConfig) {
      return;
    }
    setError(null);
    setMessage(null);
    try {
      await Promise.all([
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
        saveUiConfig(uiConfig),
      ]);
      await hydrate();
      await refreshUiRuntime();
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
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <StatusNotice message={message} tone="success" onDismiss={() => setMessage(null)} />
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
              disabled={!config || !uiConfig}
            >
              {t("web.settings.save_system", "保存系统设置")}
            </button>
          </div>
        </div>
      </section>

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

        {config && uiConfig ? (
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
              id="settings-model-endpoints"
              className="settings-model-endpoints-shell settings-module settings-section-anchor"
            >
              <ModelEndpointsPage embedded />
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

            <article className="mini-card settings-section-card settings-section-anchor settings-module">
              <div className="settings-section-card__header">
                <div className="panel-title">内置工具</div>
                <p className="helper-text">命令执行和网络请求的默认超时与上下限。</p>
              </div>
              <div className="form-grid settings-form-grid">
                <label className="settings-field">
                  命令默认超时
                  <input
                    value={config.operations.command.default_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        operations: {
                          ...config.operations,
                          command: {
                            ...config.operations.command,
                            default_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  命令最小超时
                  <input
                    value={config.operations.command.min_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        operations: {
                          ...config.operations,
                          command: {
                            ...config.operations.command,
                            min_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  命令最大超时
                  <input
                    value={config.operations.command.max_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        operations: {
                          ...config.operations,
                          command: {
                            ...config.operations.command,
                            max_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  网络默认超时
                  <input
                    value={config.operations.net.default_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        operations: {
                          ...config.operations,
                          net: {
                            ...config.operations.net,
                            default_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  网络最小超时
                  <input
                    value={config.operations.net.min_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        operations: {
                          ...config.operations,
                          net: {
                            ...config.operations.net,
                            min_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  网络最大超时
                  <input
                    value={config.operations.net.max_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        operations: {
                          ...config.operations,
                          net: {
                            ...config.operations.net,
                            max_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
              </div>
            </article>

            <article className="mini-card settings-section-card settings-section-anchor settings-module">
              <div className="settings-section-card__header">
                <div className="panel-title">上游与流式</div>
                <p className="helper-text">控制上游请求超时，以及会话、工作流、日志流的轮询间隔。</p>
              </div>
              <div className="form-grid settings-form-grid">
                <label className="settings-field">
                  上游默认超时
                  <input
                    value={config.providers.default_request_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        providers: {
                          ...config.providers,
                          default_request_timeout_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  会话流间隔
                  <input
                    value={config.streams.conversation_poll_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        streams: {
                          ...config.streams,
                          conversation_poll_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  工作流流间隔
                  <input
                    value={config.streams.workflow_poll_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        streams: {
                          ...config.streams,
                          workflow_poll_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  日志流间隔
                  <input
                    value={config.streams.logs_poll_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        streams: {
                          ...config.streams,
                          logs_poll_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
              </div>
            </article>

            <article className="mini-card settings-section-card settings-section-anchor settings-module">
              <div className="settings-section-card__header">
                <div className="panel-title">后台循环</div>
                <p className="helper-text">控制扩展刷新、计划扫描和事件投递这些后台循环的运行节奏。</p>
              </div>
              <div className="form-grid settings-form-grid">
                <label className="settings-field">
                  扩展刷新间隔
                  <input
                    value={config.background.extension_refresh_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        background: {
                          ...config.background,
                          extension_refresh_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  计划扫描间隔
                  <input
                    value={config.background.schedule_tick_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        background: {
                          ...config.background,
                          schedule_tick_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  事件投递间隔
                  <input
                    value={config.background.event_delivery_tick_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        background: {
                          ...config.background,
                          event_delivery_tick_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
              </div>
            </article>

            <article className="mini-card settings-section-card settings-section-anchor settings-module">
              <div className="settings-section-card__header">
                <div className="panel-title">扩展运行时</div>
                <p className="helper-text">给没有显式声明 runtime 配额的扩展提供宿主默认值。</p>
              </div>
              <div className="form-grid settings-form-grid">
                <label className="settings-field">
                  Worker 默认超时
                  <input
                    value={config.extension_runtime.timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        extension_runtime: {
                          ...config.extension_runtime,
                          timeout_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  Worker 默认内存
                  <input
                    value={config.extension_runtime.memory_limit_mb}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        extension_runtime: {
                          ...config.extension_runtime,
                          memory_limit_mb: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
              </div>
            </article>

            <article className="mini-card settings-section-card settings-section-anchor settings-module">
              <div className="settings-section-card__header">
                <div className="panel-title">调度默认值</div>
                <p className="helper-text">统一调度命令的超时范围和重试策略边界。</p>
              </div>
              <div className="form-grid settings-form-grid">
                <label className="settings-field">
                  调度命令默认超时
                  <input
                    value={config.schedules.command.default_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        schedules: {
                          ...config.schedules,
                          command: {
                            ...config.schedules.command,
                            default_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  调度命令最小超时
                  <input
                    value={config.schedules.command.min_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        schedules: {
                          ...config.schedules,
                          command: {
                            ...config.schedules.command,
                            min_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  调度命令最大超时
                  <input
                    value={config.schedules.command.max_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        schedules: {
                          ...config.schedules,
                          command: {
                            ...config.schedules.command,
                            max_timeout_ms: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  默认重试次数
                  <input
                    value={config.schedules.retry.default_max_attempts}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        schedules: {
                          ...config.schedules,
                          retry: {
                            ...config.schedules.retry,
                            default_max_attempts: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  最大重试次数
                  <input
                    value={config.schedules.retry.max_attempts_cap}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        schedules: {
                          ...config.schedules,
                          retry: {
                            ...config.schedules.retry,
                            max_attempts_cap: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  默认重试间隔
                  <input
                    value={config.schedules.retry.default_backoff_seconds}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        schedules: {
                          ...config.schedules,
                          retry: {
                            ...config.schedules.retry,
                            default_backoff_seconds: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  最大重试间隔
                  <input
                    value={config.schedules.retry.max_backoff_seconds}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        schedules: {
                          ...config.schedules,
                          retry: {
                            ...config.schedules.retry,
                            max_backoff_seconds: Number(event.target.value),
                          },
                        },
                      })
                    }
                  />
                </label>
              </div>
            </article>

            <article className="mini-card settings-section-card settings-section-anchor settings-module">
              <div className="settings-section-card__header">
                <div className="panel-title">前端运行时</div>
                <p className="helper-text">控制前端 API 请求默认超时，以及全局弹窗的自动消失行为。</p>
              </div>
              <div className="form-grid settings-form-grid">
                <label className="settings-field">
                  前端 API 默认超时
                  <input
                    value={uiConfig.api.default_request_timeout_ms ?? ""}
                    placeholder="留空表示不在前端层强制超时"
                    onChange={(event) =>
                      setUiConfig({
                        ...uiConfig,
                        api: {
                          ...uiConfig.api,
                          default_request_timeout_ms: event.target.value.trim()
                            ? Number(event.target.value)
                            : null,
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  成功提示自动消失
                  <input
                    value={uiConfig.notifications.success_auto_dismiss_ms}
                    onChange={(event) =>
                      setUiConfig({
                        ...uiConfig,
                        notifications: {
                          ...uiConfig.notifications,
                          success_auto_dismiss_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  错误提示自动消失
                  <input
                    value={uiConfig.notifications.error_auto_dismiss_ms}
                    onChange={(event) =>
                      setUiConfig({
                        ...uiConfig,
                        notifications: {
                          ...uiConfig.notifications,
                          error_auto_dismiss_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="check-row settings-toggle-row settings-field settings-field--wide">
                  <input
                    type="checkbox"
                    checked={uiConfig.notifications.pause_on_hover}
                    onChange={(event) =>
                      setUiConfig({
                        ...uiConfig,
                        notifications: {
                          ...uiConfig.notifications,
                          pause_on_hover: event.target.checked,
                        },
                      })
                    }
                  />
                  悬停时暂停自动消失
                </label>
              </div>
            </article>

            <article className="mini-card settings-section-card settings-section-anchor settings-module">
              <div className="settings-section-card__header">
                <div className="panel-title">开发态高级</div>
                <p className="helper-text">只影响 `ennoia dev` 这类本地开发链路，不影响生产服务运行。</p>
              </div>
              <div className="form-grid settings-form-grid">
                <label className="settings-field">
                  Host reload debounce
                  <input
                    value={config.dev_supervisor.host_reload_debounce_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        dev_supervisor: {
                          ...config.dev_supervisor,
                          host_reload_debounce_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  Watch poll
                  <input
                    value={config.dev_supervisor.watch_poll_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        dev_supervisor: {
                          ...config.dev_supervisor,
                          watch_poll_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  API ready timeout
                  <input
                    value={config.dev_supervisor.api_ready_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        dev_supervisor: {
                          ...config.dev_supervisor,
                          api_ready_timeout_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  健康检查间隔
                  <input
                    value={config.dev_supervisor.api_healthcheck_interval_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        dev_supervisor: {
                          ...config.dev_supervisor,
                          api_healthcheck_interval_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  健康检查宽限
                  <input
                    value={config.dev_supervisor.api_healthcheck_grace_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        dev_supervisor: {
                          ...config.dev_supervisor,
                          api_healthcheck_grace_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  端口释放等待
                  <input
                    value={config.dev_supervisor.api_port_release_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        dev_supervisor: {
                          ...config.dev_supervisor,
                          api_port_release_timeout_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  子进程启动宽限
                  <input
                    value={config.dev_supervisor.child_startup_grace_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        dev_supervisor: {
                          ...config.dev_supervisor,
                          child_startup_grace_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
                <label className="settings-field">
                  Probe socket timeout
                  <input
                    value={config.dev_supervisor.probe_socket_timeout_ms}
                    onChange={(event) =>
                      setConfig({
                        ...config,
                        dev_supervisor: {
                          ...config.dev_supervisor,
                          probe_socket_timeout_ms: Number(event.target.value),
                        },
                      })
                    }
                  />
                </label>
              </div>
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
