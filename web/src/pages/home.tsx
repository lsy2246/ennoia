export function Home() {
  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center" }}>
      <div style={{
        width: "120px",
        height: "120px",
        borderRadius: "28px",
        background: "linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 70%, transparent))",
        boxShadow: "0 12px 32px rgba(0, 122, 255, 0.2)",
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        marginBottom: "32px",
        color: "white",
        fontSize: "64px",
        fontWeight: "200",
      }}>
        E
      </div>
      <h1 style={{ fontSize: "36px", marginBottom: "12px", fontWeight: "600", letterSpacing: "-0.02em", color: "var(--text)" }}>Welcome to Ennoia</h1>
      <p style={{ fontSize: "16px", color: "var(--text-muted)", maxWidth: "360px", textAlign: "center", lineHeight: "1.5" }}>
        Select an item from the sidebar to start a conversation, manage agents, or configure extensions.
      </p>
    </div>
  );
}
