import { builtinExtensionPages, builtinExtensionPanels } from "@ennoia/builtins";

import { PageHeader } from "@/components/PageHeader";
import { useWorkspaceSnapshot } from "@/hooks/useWorkspaceSnapshot";
import { useUiHelpers } from "@/stores/ui";

export function ExtensionsPage() {
  const { snapshot, loading, error, refresh } = useWorkspaceSnapshot();
  const { t, resolveText } = useUiHelpers();

  if (loading || !snapshot) {
    return <div className="page"><p>{t("shell.loading.extensions", "Loading extensions…")}</p></div>;
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.extensions.title", "Extensions")}
        description={t(
          "shell.page.extensions.description",
          "Inspect installed extensions, contributed pages/panels/themes/locales and skill directories.",
        )}
        meta={[
          `${snapshot.registry.extensions.length} ${t("shell.extensions.installed", "installed")}`,
          `${snapshot.registry.pages.length} ${t("shell.extensions.pages", "pages")}`,
          `${snapshot.registry.panels.length} ${t("shell.extensions.panels", "panels")}`,
        ]}
        actions={
          <button className="secondary" onClick={() => void refresh()}>
            {t("shell.action.refresh", "Refresh")}
          </button>
        }
      />

      {error && <div className="error">{error}</div>}

      <section>
        <h2>{t("shell.extensions.registry", "Installed extensions")}</h2>
        <table className="table">
          <thead>
            <tr>
              <th>ID</th>
              <th>{t("shell.extensions.kind", "Kind")}</th>
              <th>{t("shell.extensions.version", "Version")}</th>
              <th>{t("shell.extensions.install_dir", "Install dir")}</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.registry.extensions.map((extension) => (
              <tr key={extension.id}>
                <td><code>{extension.id}</code></td>
                <td>{extension.kind}</td>
                <td>{extension.version}</td>
                <td><code>{extension.install_dir}</code></td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="split-grid">
        <div>
          <h2>{t("shell.extensions.pages", "Pages")}</h2>
          <table className="table">
            <thead>
              <tr>
                <th>{t("shell.extensions.title", "Title")}</th>
                <th>Route</th>
                <th>Mount</th>
              </tr>
            </thead>
            <tbody>
              {snapshot.registry.pages.map((page) => {
                const builtin = builtinExtensionPages[page.page.mount];
                return (
                  <tr key={page.page.id}>
                    <td>
                      <strong>{resolveText(page.page.title)}</strong>
                      <div className="muted">{builtin?.summary ?? page.extension_id}</div>
                    </td>
                    <td><code>{page.page.route}</code></td>
                    <td><code>{page.page.mount}</code></td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>

        <div>
          <h2>{t("shell.extensions.panels", "Panels")}</h2>
          <table className="table">
            <thead>
              <tr>
                <th>{t("shell.extensions.title", "Title")}</th>
                <th>{t("shell.extensions.slot", "Slot")}</th>
                <th>Mount</th>
              </tr>
            </thead>
            <tbody>
              {snapshot.registry.panels.map((panel) => {
                const builtin = builtinExtensionPanels[panel.panel.mount];
                return (
                  <tr key={panel.panel.id}>
                    <td>
                      <strong>{resolveText(panel.panel.title)}</strong>
                      <div className="muted">{builtin?.summary ?? panel.extension_id}</div>
                    </td>
                    <td>{panel.panel.slot}</td>
                    <td><code>{panel.panel.mount}</code></td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </section>

      <section className="split-grid">
        <div>
          <h2>{t("shell.extensions.themes", "Theme contributions")}</h2>
          <table className="table">
            <thead>
              <tr>
                <th>ID</th>
                <th>{t("shell.extensions.title", "Title")}</th>
                <th>{t("shell.extensions.kind", "Appearance")}</th>
              </tr>
            </thead>
            <tbody>
              {snapshot.registry.themes.map((theme) => (
                <tr key={theme.theme.id}>
                  <td><code>{theme.theme.id}</code></td>
                  <td>{resolveText(theme.theme.label)}</td>
                  <td>{theme.theme.appearance}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div>
          <h2>{t("shell.extensions.locales", "Locale contributions")}</h2>
          <table className="table">
            <thead>
              <tr>
                <th>Locale</th>
                <th>Namespace</th>
                <th>{t("shell.extensions.version", "Version")}</th>
              </tr>
            </thead>
            <tbody>
              {snapshot.registry.locales.map((locale) => (
                <tr key={`${locale.locale.locale}:${locale.locale.namespace}`}>
                  <td>{locale.locale.locale}</td>
                  <td><code>{locale.locale.namespace}</code></td>
                  <td>{locale.locale.version}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section>
        <h2>{t("shell.extensions.skills", "Skill directories")}</h2>
        <table className="table">
          <thead>
            <tr>
              <th>{t("shell.owner.agent", "Agent")}</th>
              <th>{t("shell.extensions.skills", "Skills dir")}</th>
              <th>{t("shell.extensions.install_dir", "Workspace dir")}</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.agents.map((agent) => (
              <tr key={agent.id}>
                <td>{agent.display_name}</td>
                <td><code>{agent.skills_dir}</code></td>
                <td><code>{agent.workspace_dir}</code></td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
