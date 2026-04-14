type Page = {
  id: string;
  title: string;
  description: string;
};

type Panel = {
  id: string;
  title: string;
  summary: string;
};

const pages: Page[] = [
  { id: "inbox", title: "私聊收件箱", description: "查看和某个 Agent 的私聊线程。" },
  { id: "spaces", title: "群聊空间", description: "查看多 Agent 协作的 Space 与线程。" },
  { id: "runs", title: "运行编排", description: "查看 run、task、gate 和 artifacts。" },
];

const panels: Panel[] = [
  { id: "inspector", title: "Run Inspector", summary: "显示选中 run 的 owner、状态和任务。" },
  { id: "memory", title: "Memory View", summary: "显示上下文视图和 recall 摘要。" },
  { id: "logs", title: "Logs", summary: "显示扩展、调度和服务的结构化事件流。" },
];

export function App() {
  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <p className="eyebrow">AI Workbench</p>
          <h1>Ennoia</h1>
          <p className="muted">Shell + Child Pages + Panels</p>
        </div>

        <nav className="nav">
          {pages.map((page) => (
            <button key={page.id} className="nav-item" type="button">
              <span>{page.title}</span>
            </button>
          ))}
        </nav>
      </aside>

      <main className="content">
        <header className="header">
          <div>
            <p className="eyebrow">Workspace</p>
            <h2>主壳预览</h2>
          </div>
          <div className="status-pill">observatory registered</div>
        </header>

        <section className="page-grid">
          {pages.map((page) => (
            <article key={page.id} className="page-card">
              <p className="eyebrow">Child Page</p>
              <h3>{page.title}</h3>
              <p>{page.description}</p>
            </article>
          ))}
        </section>

        <section className="dock">
          <div className="dock-header">
            <div>
              <p className="eyebrow">Panels</p>
              <h3>可拖拽面板区域</h3>
            </div>
            <p className="muted">下一步接入 Dockview 和布局持久化。</p>
          </div>

          <div className="panel-grid">
            {panels.map((panel) => (
              <article key={panel.id} className="panel-card">
                <h4>{panel.title}</h4>
                <p>{panel.summary}</p>
              </article>
            ))}
          </div>
        </section>
      </main>
    </div>
  );
}
