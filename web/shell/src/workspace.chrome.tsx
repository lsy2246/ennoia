import { builtinExtensionPages, builtinExtensionPanels } from "../../builtins/src";
import type { Artifact, Message, Task, Thread, WorkspaceSnapshot } from "./api";
import { styles } from "./app.styles";
import {
  fallbackPageDescriptor,
  fallbackPanelDescriptor,
  panelMetric,
  type ViewMode,
} from "./workspace.helpers";
import {
  EmptyState,
  HeroMetric,
  ListRow,
  NavButton,
  StatCard,
  ThreadButton,
} from "./workspace.primitives";

type SidebarProps = {
  activeView: ViewMode;
  onChangeView: (view: ViewMode) => void;
  onSelectThread: (threadId: string, view: ViewMode) => void;
  privateThreads: Thread[];
  selectedThreadId: string;
  shellTitle: string;
  spaceThreads: Thread[];
  stats: Record<string, number>;
};

type HeroSectionProps = {
  appName: string;
  artifactCount: number;
  memoryCount: number;
  panelCount: number;
  status: string;
  taskCount: number;
  onOpenExtensions: () => void;
  onRefresh: () => void;
};

type ExtensionDockProps = {
  activePageMount: string;
  artifacts: Artifact[];
  extensionPages: WorkspaceSnapshot["registry"]["pages"];
  groupedPanels: Record<string, WorkspaceSnapshot["registry"]["panels"][number][]>;
  messages: Message[];
  runs: WorkspaceSnapshot["runs"];
  selectedPageMount: string;
  setSelectedPageMount: (mount: string) => void;
  setView: (view: ViewMode) => void;
  tasks: Task[];
};

export function Sidebar(props: SidebarProps) {
  return (
    <aside className={styles.sidebar}>
      <div className={styles.brandBlock}>
        <p className={styles.overline}>Signal Atlas</p>
        <h1 className={styles.brandTitle}>{props.shellTitle}</h1>
        <p className={styles.muted}>把私聊、群聊、运行态和扩展挂载收进同一张工作台地图。</p>
      </div>

      <div className={styles.sidebarSection}>
        <div className={styles.statGrid}>
          <StatCard label="Threads" value={props.stats.threads ?? 0} />
          <StatCard label="Runs" value={props.stats.runs ?? 0} />
          <StatCard label="Messages" value={props.stats.messages ?? 0} />
          <StatCard label="Extensions" value={props.stats.extensions ?? 0} />
        </div>
      </div>

      <div className={styles.sidebarSection}>
        <p className={styles.overline}>Workspace View</p>
        <div className={styles.navList}>
          <NavButton
            active={props.activeView === "private"}
            onClick={() => props.onChangeView("private")}
          >
            私聊指挥台
          </NavButton>
          <NavButton
            active={props.activeView === "space"}
            onClick={() => props.onChangeView("space")}
          >
            群聊协作台
          </NavButton>
          <NavButton
            active={props.activeView === "extensions"}
            onClick={() => props.onChangeView("extensions")}
          >
            Extension Surface
          </NavButton>
        </div>
      </div>

      <div className={styles.panelStack}>
        <aside className={styles.sidebarCard}>
          <p className={styles.overline}>Private Threads</p>
          <div className={styles.threadList}>
            {props.privateThreads.map((thread) => (
              <ThreadButton
                key={thread.id}
                active={props.selectedThreadId === thread.id}
                thread={thread}
                onClick={() => props.onSelectThread(thread.id, "private")}
              />
            ))}
          </div>
        </aside>

        <aside className={styles.sidebarCard}>
          <p className={styles.overline}>Space Threads</p>
          <div className={styles.threadList}>
            {props.spaceThreads.map((thread) => (
              <ThreadButton
                key={thread.id}
                active={props.selectedThreadId === thread.id}
                thread={thread}
                onClick={() => props.onSelectThread(thread.id, "space")}
              />
            ))}
          </div>
        </aside>
      </div>
    </aside>
  );
}

export function HeroSection(props: HeroSectionProps) {
  return (
    <section className={styles.hero}>
      <div className={styles.heroGrid}>
        <div className={styles.brandBlock}>
          <p className={styles.overline}>AI Workspace Runtime</p>
          <h2 className={styles.heroTitle}>
            {props.appName} 把会话、执行、记忆和扩展收拢进统一界面。
          </h2>
          <p className={styles.heroSummary}>
            当前主壳已经接上正式 `thread / message / run / task / artifact / memory`
            API，也会按 registry 协议展示 extension page 与 panel 的挂载位。
          </p>
        </div>

        <div className={styles.heroMetrics}>
          <HeroMetric label="Tasks" value={props.taskCount} />
          <HeroMetric label="Artifacts" value={props.artifactCount} />
          <HeroMetric label="Memories" value={props.memoryCount} />
          <HeroMetric label="Panels" value={props.panelCount} />
        </div>
      </div>

      <div className={styles.actionRow}>
        <button className={styles.primaryButton} type="button" onClick={props.onRefresh}>
          刷新工作台
        </button>
        <button
          className={styles.secondaryButton}
          type="button"
          onClick={props.onOpenExtensions}
        >
          查看 Extension Surface
        </button>
        <span className={styles.statusPill}>{props.status}</span>
      </div>
    </section>
  );
}

export function ExtensionDock(props: ExtensionDockProps) {
  const activePage =
    props.extensionPages.find((page) => page.page.mount === props.selectedPageMount) ??
    props.extensionPages[0];
  const activePageMeta =
    (activePage && builtinExtensionPages[activePage.page.mount]) ??
    fallbackPageDescriptor(activePage);

  return (
    <section className={styles.extensionDock}>
      <div className={styles.sectionHeader}>
        <div className={styles.brandBlock}>
          <p className={styles.overline}>Extension Surface</p>
          <h3 className={styles.sectionTitle}>按 registry 协议挂载 page 与 panel 容器</h3>
          <p className={styles.sectionCopy}>
            page 和 panel 都由后端 registry 驱动，主壳只消费 mount 协议与内建描述。
          </p>
        </div>
        <div className={styles.chipRow}>
          <span className={styles.chip}>{props.extensionPages.length} pages</span>
          <span className={styles.chip}>
            {Object.values(props.groupedPanels).flat().length} panels
          </span>
        </div>
      </div>

      <div className={styles.extensionTabs}>
        {props.extensionPages.map((page) => (
          <button
            key={page.page.mount}
            className={`${styles.extensionTab} ${
              props.activePageMount === page.page.mount ? styles.extensionTabActive : ""
            }`}
            type="button"
            onClick={() => {
              props.setView("extensions");
              props.setSelectedPageMount(page.page.mount);
            }}
          >
            {page.page.title}
          </button>
        ))}
      </div>

      <div className={styles.extensionGrid}>
        <article className={styles.extensionSurface}>
          {activePage ? (
            <>
              <div className={styles.extensionHero}>
                <p className={styles.overline}>{activePageMeta.eyebrow}</p>
                <h4 className={styles.extensionTitle}>{activePage.page.title}</h4>
                <p className={styles.sectionCopy}>{activePageMeta.summary}</p>
              </div>

              <div className={styles.highlightList}>
                {activePageMeta.highlights.map((highlight) => (
                  <span key={highlight} className={styles.chip}>
                    {highlight}
                  </span>
                ))}
              </div>

              <div className={styles.panelStack}>
                <ListRow title="Route" meta={activePage.page.route} />
                <ListRow title="Mount" meta={activePage.page.mount} />
                <ListRow
                  title="Source"
                  meta={`${activePage.extension_id} · ${activePage.extension_version}`}
                />
              </div>
            </>
          ) : (
            <EmptyState text="当前没有可挂载的 extension page。" />
          )}
        </article>

        <div className={styles.panelSlotGrid}>
          {Object.entries(props.groupedPanels).map(([slot, panels]) => (
            <section key={slot} className={styles.panelSlot}>
              <h4 className={styles.panelSlotTitle}>{slot.toUpperCase()} Slot</h4>
              {panels.length > 0 ? (
                panels.map((panel) => {
                  const builtin =
                    builtinExtensionPanels[panel.panel.mount] ??
                    fallbackPanelDescriptor(panel.panel.mount, slot);

                  return (
                    <article key={panel.panel.mount} className={styles.extensionPanelCard}>
                      <strong>{panel.panel.title}</strong>
                      <span className={styles.listMeta}>{builtin.summary}</span>
                      <span className={styles.listMeta}>
                        {builtin.metricLabel}:{" "}
                        {panelMetric(
                          panel.panel.mount,
                          props.messages,
                          props.runs,
                          props.tasks,
                          props.artifacts,
                        )}
                      </span>
                    </article>
                  );
                })
              ) : (
                <EmptyState text={`当前没有挂载到 ${slot} 的 panel。`} />
              )}
            </section>
          ))}
        </div>
      </div>
    </section>
  );
}
