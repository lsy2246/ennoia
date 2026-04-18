import {
  startTransition,
  useDeferredValue,
  useEffect,
  useState,
  type FormEvent,
} from "react";

import { groupPanelsBySlot, sortExtensionPages } from "../../ui-sdk/src";
import {
  createJob,
  loadThreadMessages,
  loadWorkspaceSnapshot,
  sendPrivateMessage,
  sendSpaceMessage,
  type Message,
} from "./api";
import { styles } from "./app.styles";
import {
  ActivityRail,
  ConversationSection,
  ExtensionDock,
  HeroSection,
  Sidebar,
} from "./workspace.components";
import {
  buildJobDescription,
  buildPrivateDraft,
  buildSpaceDraft,
  emptySnapshot,
  pickThreadId,
  reconcileSelectedSpaceAgents,
  type ViewMode,
} from "./workspace.helpers";

export function App() {
  const [snapshot, setSnapshot] = useState(emptySnapshot);
  const [messagesByThread, setMessagesByThread] = useState<Record<string, Message[]>>({});
  const [status, setStatus] = useState("正在连接后端工作台");
  const [activeView, setActiveView] = useState<ViewMode>("private");
  const [selectedThreadId, setSelectedThreadId] = useState("");
  const [selectedPageMount, setSelectedPageMount] = useState("");
  const [selectedSpaceId, setSelectedSpaceId] = useState("");
  const [selectedSpaceAgents, setSelectedSpaceAgents] = useState<string[]>([]);
  const [privateAgentId, setPrivateAgentId] = useState("");
  const [privateGoal, setPrivateGoal] = useState("");
  const [privateBody, setPrivateBody] = useState("");
  const [spaceGoal, setSpaceGoal] = useState("");
  const [spaceBody, setSpaceBody] = useState("");
  const [jobDescription, setJobDescription] = useState("");
  const deferredThreadId = useDeferredValue(selectedThreadId);

  const privateThreads = snapshot.threads.filter((thread) => thread.kind === "Private");
  const spaceThreads = snapshot.threads.filter((thread) => thread.kind === "Space");
  const selectedThread =
    snapshot.threads.find((thread) => thread.id === deferredThreadId) ??
    privateThreads[0] ??
    spaceThreads[0];
  const selectedMessages = selectedThread ? messagesByThread[selectedThread.id] ?? [] : [];
  const selectedRuns = snapshot.runs.filter((run) => run.thread_id === selectedThread?.id);
  const selectedRunIds = new Set(selectedRuns.map((run) => run.id));
  const selectedTasks = snapshot.tasks.filter((task) => selectedRunIds.has(task.run_id));
  const selectedArtifacts = snapshot.artifacts.filter((artifact) =>
    selectedRunIds.has(artifact.run_id),
  );
  const selectedMemories = snapshot.memories.filter(
    (memory) =>
      memory.thread_id === selectedThread?.id ||
      (memory.run_id ? selectedRunIds.has(memory.run_id) : false),
  );
  const extensionPages = sortExtensionPages(snapshot.registry.pages);
  const groupedPanels = groupPanelsBySlot(snapshot.registry.panels);
  const selectedSpace =
    snapshot.spaces.find((space) => space.id === selectedSpaceId) ?? snapshot.spaces[0];
  const selectedPrivateAgent =
    snapshot.agents.find((agent) => agent.id === privateAgentId) ?? snapshot.agents[0];
  const addressedAgents = snapshot.agents.filter((agent) =>
    selectedSpaceAgents.includes(agent.id),
  );
  const privateDraft = buildPrivateDraft(selectedPrivateAgent);
  const spaceDraft = buildSpaceDraft(selectedSpace, addressedAgents);
  const jobDraft = buildJobDescription(selectedSpace);

  useEffect(() => {
    void refreshWorkspace();
  }, []);

  useEffect(() => {
    if (!deferredThreadId || messagesByThread[deferredThreadId]) {
      return;
    }

    void syncThreadMessages(deferredThreadId);
  }, [deferredThreadId, messagesByThread]);

  useEffect(() => {
    if (!snapshot.spaces.some((space) => space.id === selectedSpaceId)) {
      setSelectedSpaceId(snapshot.spaces[0]?.id ?? "");
    }

    if (!snapshot.agents.some((agent) => agent.id === privateAgentId)) {
      setPrivateAgentId(snapshot.agents[0]?.id ?? "");
    }
  }, [privateAgentId, selectedSpaceId, snapshot.agents, snapshot.spaces]);

  useEffect(() => {
    const nextAgents = reconcileSelectedSpaceAgents(
      selectedSpace,
      snapshot.agents,
      selectedSpaceAgents,
    );

    const changed =
      nextAgents.length !== selectedSpaceAgents.length ||
      nextAgents.some((agentId, index) => selectedSpaceAgents[index] !== agentId);

    if (changed) {
      setSelectedSpaceAgents(nextAgents);
    }
  }, [selectedSpace, selectedSpaceAgents, snapshot.agents]);

  async function refreshWorkspace(preferredThreadId?: string, preferredPageMount?: string) {
    try {
      const next = await loadWorkspaceSnapshot();
      startTransition(() => {
        setSnapshot(next);
      });

      const nextThreadId = pickThreadId(
        next.threads,
        preferredThreadId ?? selectedThreadId,
        activeView,
      );
      const nextPageMount =
        preferredPageMount ??
        (next.registry.pages.some((page) => page.page.mount === selectedPageMount)
          ? selectedPageMount
          : next.registry.pages[0]?.page.mount ?? "");

      setSelectedThreadId(nextThreadId);
      setSelectedPageMount(nextPageMount);

      if (nextThreadId) {
        void syncThreadMessages(nextThreadId);
      }

      setStatus("工作台已同步");
    } catch (error) {
      setStatus(`工作台连接失败：${String(error)}`);
    }
  }

  async function syncThreadMessages(threadId: string) {
    try {
      const nextMessages = await loadThreadMessages(threadId);
      startTransition(() => {
        setMessagesByThread((current) => ({
          ...current,
          [threadId]: nextMessages,
        }));
      });
    } catch (error) {
      setStatus(`消息时间线加载失败：${String(error)}`);
    }
  }

  async function submitPrivateMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const envelope = await sendPrivateMessage({
      agent_id: privateAgentId,
      body: privateBody.trim() || privateDraft.body,
      goal: privateGoal.trim() || privateDraft.goal,
    });
    setActiveView("private");
    setStatus(`已投递私聊线程 ${envelope.thread.title}`);
    setMessagesByThread((current) => ({
      ...current,
      [envelope.thread.id]: [...(current[envelope.thread.id] ?? []), envelope.message],
    }));
    await refreshWorkspace(envelope.thread.id);
  }

  async function submitSpaceMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const addressedAgentIds = reconcileSelectedSpaceAgents(
      selectedSpace,
      snapshot.agents,
      selectedSpaceAgents,
    );
    const envelope = await sendSpaceMessage({
      space_id: selectedSpaceId,
      addressed_agents: addressedAgentIds,
      body: spaceBody.trim() || spaceDraft.body,
      goal: spaceGoal.trim() || spaceDraft.goal,
    });
    setActiveView("space");
    setStatus(`已投递群聊线程 ${envelope.thread.title}`);
    setMessagesByThread((current) => ({
      ...current,
      [envelope.thread.id]: [...(current[envelope.thread.id] ?? []), envelope.message],
    }));
    await refreshWorkspace(envelope.thread.id);
  }

  async function submitJob(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await createJob({
      owner_kind: "space",
      owner_id: selectedSpaceId,
      schedule_kind: "cron",
      schedule_value: "0 */6 * * *",
      description: jobDescription.trim() || jobDraft,
    });
    setStatus(`已注册 ${jobDescription.trim() || jobDraft}`);
    await refreshWorkspace();
  }

  return (
    <div className={styles.shell}>
      <div className={styles.backdrop} />
      <div className={styles.layout}>
        <Sidebar
          activeView={activeView}
          onChangeView={setActiveView}
          onSelectThread={(threadId, view) => {
            setActiveView(view);
            setSelectedThreadId(threadId);
          }}
          privateThreads={privateThreads}
          selectedThreadId={selectedThread?.id ?? ""}
          shellTitle={snapshot.overview.shell_title}
          spaceThreads={spaceThreads}
          stats={snapshot.overview.counts}
        />

        <main className={styles.main}>
          <HeroSection
            appName={snapshot.overview.app_name}
            artifactCount={snapshot.overview.counts.artifacts ?? 0}
            memoryCount={snapshot.overview.counts.memories ?? 0}
            panelCount={snapshot.registry.panels.length}
            status={status}
            taskCount={snapshot.overview.counts.tasks ?? 0}
            onOpenExtensions={() => setActiveView("extensions")}
            onRefresh={() => void refreshWorkspace()}
          />

          <section className={styles.stageGrid}>
            <ConversationSection
              agents={snapshot.agents}
              jobDescription={jobDescription}
              jobPlaceholder={jobDraft}
              messages={selectedMessages}
              privateAgentId={privateAgentId}
              privateBody={privateBody}
              privateGoal={privateGoal}
              privatePlaceholders={privateDraft}
              runs={selectedRuns}
              selectedSpaceAgents={selectedSpaceAgents}
              selectedSpaceId={selectedSpaceId}
              selectedThread={selectedThread}
              setJobDescription={setJobDescription}
              setPrivateAgentId={setPrivateAgentId}
              setPrivateBody={setPrivateBody}
              setPrivateGoal={setPrivateGoal}
              setSelectedSpaceAgents={setSelectedSpaceAgents}
              setSelectedSpaceId={setSelectedSpaceId}
              setSpaceBody={setSpaceBody}
              setSpaceGoal={setSpaceGoal}
              spaceBody={spaceBody}
              spaceGoal={spaceGoal}
              spacePlaceholders={spaceDraft}
              spaces={snapshot.spaces}
              submitJob={submitJob}
              submitPrivateMessage={submitPrivateMessage}
              submitSpaceMessage={submitSpaceMessage}
              tasks={selectedTasks}
            />

            <ActivityRail
              artifacts={selectedArtifacts}
              jobs={snapshot.jobs}
              memories={selectedMemories}
              runs={selectedRuns}
              tasks={selectedTasks}
            />
          </section>

          <ExtensionDock
            activePageMount={selectedPageMount}
            artifacts={selectedArtifacts}
            extensionPages={extensionPages}
            groupedPanels={groupedPanels}
            messages={selectedMessages}
            runs={selectedRuns}
            selectedPageMount={selectedPageMount}
            setSelectedPageMount={setSelectedPageMount}
            setView={setActiveView}
            tasks={selectedTasks}
          />
        </main>
      </div>
    </div>
  );
}
