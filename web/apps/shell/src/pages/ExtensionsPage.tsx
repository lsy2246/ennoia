import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import {
  attachExtensionWorkspace,
  listExtensions,
  reloadExtension,
  restartExtension,
  setExtensionEnabled,
  type ExtensionRuntimeState,
} from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useUiHelpers } from "@/stores/ui";

export function ExtensionsPage() {
  const { t } = useUiHelpers();
  const [items, setItems] = useState<ExtensionRuntimeState[]>([]);
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      setItems(await listExtensions());
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  return (
    <div className="page">
      <PageHeader
        title={t("shell.extensions.page_title", "扩展")}
        description={t(
          "shell.extensions.page_description",
          "扩展是正式可控能力，支持挂载开发目录、启用、停用、重载和查看诊断。",
        )}
      />

      {error ? <div className="error">{error}</div> : null}

      <section className="surface-panel">
        <div className="form-row">
          <label>
            {t("shell.extensions.attach", "挂载开发目录")}
            <input value={path} onChange={(event) => setPath(event.target.value)} />
          </label>
          <button
            onClick={() =>
              void attachExtensionWorkspace(path)
                .then(() => {
                  setPath("");
                  return refresh();
                })
                .catch((err) => setError(String(err)))
            }
          >
            {t("shell.extensions.attach_action", "挂载")}
          </button>
        </div>
      </section>

      <section className="surface-panel">
        <div className="stack-list">
          {items.map((item) => (
            <article key={item.id} className="thread-card">
              <div>
                <div className="thread-card__title">
                  <Link to="/extensions/$extensionId" params={{ extensionId: item.id }}>
                    {item.name}
                  </Link>
                </div>
                <p>
                  {item.status} · {item.source_mode}
                </p>
              </div>
              <div className="button-row">
                <button
                  className="secondary"
                  onClick={() =>
                    void setExtensionEnabled(item.id, !item.enabled)
                      .then(refresh)
                      .catch((err) => setError(String(err)))
                  }
                >
                  {item.enabled ? t("shell.extensions.disable", "停用") : t("shell.extensions.enable", "启用")}
                </button>
                <button
                  className="secondary"
                  onClick={() => void reloadExtension(item.id).then(refresh).catch((err) => setError(String(err)))}
                >
                  {t("shell.action.reload", "重载")}
                </button>
                <button
                  className="secondary"
                  onClick={() =>
                    void restartExtension(item.id).then(refresh).catch((err) => setError(String(err)))
                  }
                >
                  {t("shell.extensions.restart", "重启")}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
