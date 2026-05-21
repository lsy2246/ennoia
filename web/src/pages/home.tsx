import { useUiHelpers } from "@/stores/ui";

export function Home() {
  const { t } = useUiHelpers();

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center" }}>
      <div style={{
        width: "120px",
        height: "120px",
        borderRadius: "28px",
        background: "linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 70%, transparent))",
        boxShadow: "0 12px 32px color-mix(in srgb, var(--accent) 24%, transparent)",
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        marginBottom: "32px",
        color: "var(--bg)",
        fontSize: "64px",
        fontWeight: "200",
      }}>
        E
      </div>
      <h1 style={{ fontSize: "36px", marginBottom: "12px", fontWeight: "600", letterSpacing: "0", color: "var(--text)" }}>
        {t("web.home.title", "欢迎使用 Ennoia")}
      </h1>
      <p style={{ fontSize: "16px", color: "var(--text-muted)", maxWidth: "360px", textAlign: "center", lineHeight: "1.5" }}>
        {t("web.home.description", "从侧边栏选择入口，开始会话、管理 Agent，或配置扩展。")}
      </p>
    </div>
  );
}
