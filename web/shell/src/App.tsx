import { FormEvent, useEffect, useState } from "react";

import { styles } from "./app.styles";

type Overview = {
  app_name: string;
  shell_title: string;
  default_theme: string;
  modules: string[];
  counts: Record<string, number>;
};

type Agent = {
  id: string;
  display_name: string;
  default_model: string;
};

type Space = {
  id: string;
  display_name: string;
  default_agents: string[];
};

type Extension = {
  id: string;
  version: string;
  install_dir: string;
};

type Run = {
  id: string;
  owner_kind: string;
  owner_id: string;
  trigger: string;
  status: string;
  goal: string;
};

type Job = {
  id: string;
  owner_kind: string;
  owner_id: string;
  schedule_kind: string;
  schedule_value: string;
  description: string;
  status: string;
};

type Memory = {
  id: string;
  summary: string;
  source: string;
  owner: {
    kind: "Global" | "Agent" | "Space";
    id: string;
  };
};

const API_BASE = import.meta.env.VITE_ENNOIA_API_URL ?? "http://127.0.0.1:3710";

export function App() {
  const [overview, setOverview] = useState<Overview | null>(null);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [spaces, setSpaces] = useState<Space[]>([]);
  const [extensions, setExtensions] = useState<Extension[]>([]);
  const [runs, setRuns] = useState<Run[]>([]);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [memories, setMemories] = useState<Memory[]>([]);
  const [privateGoal, setPrivateGoal] = useState("实现一个 settings 页面");
  const [privateAgent, setPrivateAgent] = useState("coder");
  const [spaceGoal, setSpaceGoal] = useState("一起整理一份 roadmap");
  const [jobDescription, setJobDescription] = useState("nightly review");
  const [status, setStatus] = useState("等待后端连接");

  useEffect(() => {
    void loadAll();
  }, []);

  async function loadAll() {
    try {
      const [overviewRes, agentsRes, spacesRes, extensionsRes, runsRes, jobsRes, memoriesRes] =
        await Promise.all([
          fetchJson<Overview>("/api/v1/overview"),
          fetchJson<Agent[]>("/api/v1/agents"),
          fetchJson<Space[]>("/api/v1/spaces"),
          fetchJson<Extension[]>("/api/v1/extensions"),
          fetchJson<Run[]>("/api/v1/runs"),
          fetchJson<Job[]>("/api/v1/jobs"),
          fetchJson<Memory[]>("/api/v1/memories"),
        ]);

      setOverview(overviewRes);
      setAgents(agentsRes);
      setSpaces(spacesRes);
      setExtensions(extensionsRes);
      setRuns(runsRes);
      setJobs(jobsRes);
      setMemories(memoriesRes);
      setStatus("后端已连接");
    } catch (error) {
      setStatus(`连接失败：${String(error)}`);
    }
  }

  async function submitPrivateRun(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await fetchJson("/api/v1/runs/private", {
      method: "POST",
      body: JSON.stringify({
        agent_id: privateAgent,
        goal: privateGoal,
        message: privateGoal,
      }),
    });
    await loadAll();
  }

  async function submitSpaceRun(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await fetchJson("/api/v1/runs/space", {
      method: "POST",
      body: JSON.stringify({
        space_id: "studio",
        addressed_agents: ["coder", "planner"],
        goal: spaceGoal,
        message: spaceGoal,
      }),
    });
    await loadAll();
  }

  async function submitJob(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await fetchJson("/api/v1/jobs", {
      method: "POST",
      body: JSON.stringify({
        owner_kind: "space",
        owner_id: "studio",
        schedule_kind: "cron",
        schedule_value: "0 */6 * * *",
        description: jobDescription,
      }),
    });
    await loadAll();
  }

  return (
    <div className={styles.shell}>
      <aside className={styles.sidebar}>
        <div className={styles.brand}>
          <p className={styles.eyebrow}>AI Workbench</p>
          <h1 className={styles.brandTitle}>{overview?.shell_title ?? "Ennoia"}</h1>
          <p className={styles.muted}>{status}</p>
        </div>

        <div className={styles.stats}>
          <Stat label="Agents" value={overview?.counts.agents ?? 0} />
          <Stat label="Spaces" value={overview?.counts.spaces ?? 0} />
          <Stat label="Runs" value={overview?.counts.runs ?? 0} />
          <Stat label="Extensions" value={overview?.counts.extensions ?? 0} />
        </div>

        <nav className={styles.nav}>
          <button className={styles.navItem} type="button">
            私聊
          </button>
          <button className={styles.navItem} type="button">
            群聊
          </button>
          <button className={styles.navItem} type="button">
            Runs
          </button>
          <button className={styles.navItem} type="button">
            Memory
          </button>
        </nav>
      </aside>

      <main className={styles.content}>
        <header className={styles.header}>
          <div className={styles.headerBlock}>
            <p className={styles.eyebrow}>Workspace</p>
            <h2 className={styles.runtimeTitle}>{overview?.app_name ?? "Ennoia"} Runtime</h2>
            <p className={styles.muted}>
              模块：{overview?.modules.join(", ") ?? "loading"}
            </p>
          </div>
          <button className={styles.statusPill} type="button" onClick={() => void loadAll()}>
            刷新数据
          </button>
        </header>

        <section className={styles.pageGrid}>
          <form className={styles.pageCard} onSubmit={submitPrivateRun}>
            <p className={styles.eyebrow}>Private Chat</p>
            <h3 className={styles.sectionTitle}>发起私聊 Run</h3>
            <label className={styles.field}>
              <span className={styles.fieldLabel}>Agent</span>
              <select
                className={styles.fieldControl}
                value={privateAgent}
                onChange={(event) => setPrivateAgent(event.target.value)}
              >
                {agents.map((agent) => (
                  <option key={agent.id} value={agent.id}>
                    {agent.display_name}
                  </option>
                ))}
              </select>
            </label>
            <label className={styles.field}>
              <span className={styles.fieldLabel}>Goal</span>
              <input
                className={styles.fieldControl}
                value={privateGoal}
                onChange={(event) => setPrivateGoal(event.target.value)}
              />
            </label>
            <button className={styles.primaryButton} type="submit">
              创建私聊 Run
            </button>
          </form>

          <form className={styles.pageCard} onSubmit={submitSpaceRun}>
            <p className={styles.eyebrow}>Group Chat</p>
            <h3 className={styles.sectionTitle}>发起群聊 Run</h3>
            <label className={styles.field}>
              <span className={styles.fieldLabel}>Space</span>
              <select className={styles.fieldControl} defaultValue="studio">
                {spaces.map((space) => (
                  <option key={space.id} value={space.id}>
                    {space.display_name}
                  </option>
                ))}
              </select>
            </label>
            <label className={styles.field}>
              <span className={styles.fieldLabel}>Goal</span>
              <input
                className={styles.fieldControl}
                value={spaceGoal}
                onChange={(event) => setSpaceGoal(event.target.value)}
              />
            </label>
            <button className={styles.primaryButton} type="submit">
              创建群聊 Run
            </button>
          </form>

          <form className={styles.pageCard} onSubmit={submitJob}>
            <p className={styles.eyebrow}>Scheduler</p>
            <h3 className={styles.sectionTitle}>注册后台任务</h3>
            <label className={styles.field}>
              <span className={styles.fieldLabel}>Description</span>
              <input
                className={styles.fieldControl}
                value={jobDescription}
                onChange={(event) => setJobDescription(event.target.value)}
              />
            </label>
            <button className={styles.primaryButton} type="submit">
              创建 Job
            </button>
          </form>
        </section>

        <section className={styles.dock}>
          <div className={styles.dockHeader}>
            <div className={styles.headerBlock}>
              <p className={styles.eyebrow}>Panels</p>
              <h3 className={styles.sectionTitle}>运行数据面板</h3>
            </div>
            <p className={styles.muted}>当前面板已经接上真实 API 数据。</p>
          </div>

          <div className={styles.panelGrid}>
            <PanelCard title="Extensions">
              {extensions.map((extension) => (
                <ListRow key={extension.id} title={extension.id} detail={extension.version} />
              ))}
            </PanelCard>

            <PanelCard title="Runs">
              {runs.map((run) => (
                <ListRow
                  key={run.id}
                  title={run.goal}
                  detail={`${run.owner_kind}:${run.owner_id} · ${run.status}`}
                />
              ))}
            </PanelCard>

            <PanelCard title="Scheduler Jobs">
              {jobs.map((job) => (
                <ListRow
                  key={job.id}
                  title={job.description}
                  detail={`${job.schedule_kind} ${job.schedule_value}`}
                />
              ))}
            </PanelCard>

            <PanelCard title="Memories">
              {memories.map((memory) => (
                <ListRow
                  key={memory.id}
                  title={memory.summary}
                  detail={`${memory.source} · ${memory.owner.id}`}
                />
              ))}
            </PanelCard>
          </div>
        </section>
      </main>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className={styles.stat}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function PanelCard(props: { title: string; children: React.ReactNode }) {
  return (
    <article className={styles.panelCard}>
      <h4 className={styles.panelTitle}>{props.title}</h4>
      <div className={styles.list}>{props.children}</div>
    </article>
  );
}

function ListRow({ title, detail }: { title: string; detail: string }) {
  return (
    <div className={styles.listRow}>
      <strong className={styles.listTitle}>{title}</strong>
      <span className={styles.listDetail}>{detail}</span>
    </div>
  );
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      "content-type": "application/json",
    },
    ...init,
  });

  if (!response.ok) {
    throw new Error(`request failed: ${response.status}`);
  }

  return (await response.json()) as T;
}
