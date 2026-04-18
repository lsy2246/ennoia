import type { FormEvent } from "react";

import type {
  Agent,
  Artifact,
  Job,
  Memory,
  Message,
  Run,
  Space,
  Task,
  Thread,
} from "./api";
import { styles } from "./app.styles";
import { EmptyState, ListRow, PanelSection } from "./workspace.primitives";

type ConversationSectionProps = {
  agents: Agent[];
  jobDescription: string;
  jobPlaceholder: string;
  messages: Message[];
  privateAgentId: string;
  privateBody: string;
  privateGoal: string;
  privatePlaceholders: {
    body: string;
    goal: string;
  };
  runs: Run[];
  selectedSpaceAgents: string[];
  selectedThread: Thread | undefined;
  setPrivateAgentId: (value: string) => void;
  setPrivateBody: (value: string) => void;
  setPrivateGoal: (value: string) => void;
  setSelectedSpaceAgents: (value: string[]) => void;
  setSelectedSpaceId: (value: string) => void;
  setSpaceBody: (value: string) => void;
  setSpaceGoal: (value: string) => void;
  setJobDescription: (value: string) => void;
  spaces: Space[];
  spaceBody: string;
  spaceGoal: string;
  spacePlaceholders: {
    body: string;
    goal: string;
  };
  selectedSpaceId: string;
  submitJob: (event: FormEvent<HTMLFormElement>) => void;
  submitPrivateMessage: (event: FormEvent<HTMLFormElement>) => void;
  submitSpaceMessage: (event: FormEvent<HTMLFormElement>) => void;
  tasks: Task[];
};

type ActivityRailProps = {
  artifacts: Artifact[];
  jobs: Job[];
  memories: Memory[];
  runs: Run[];
  tasks: Task[];
};

export function ConversationSection(props: ConversationSectionProps) {
  return (
    <article className={styles.surfaceCard}>
      <div className={styles.sectionHeader}>
        <div className={styles.brandBlock}>
          <p className={styles.overline}>Conversation Workspace</p>
          <h3 className={styles.sectionTitle}>
            {props.selectedThread ? props.selectedThread.title : "等待线程进入工作台"}
          </h3>
          <p className={styles.sectionCopy}>
            这里承接私聊与群聊消息、run、task 和 memory 的主链路视图。
          </p>
        </div>
        <div className={styles.chipRow}>
          <span className={styles.chip}>{props.selectedThread?.kind ?? "No thread"}</span>
          <span className={styles.chip}>{props.runs.length} runs</span>
          <span className={styles.chip}>{props.tasks.length} tasks</span>
        </div>
      </div>

      <div className={styles.workspaceGrid}>
        <div className={styles.composerGrid}>
          <article className={styles.composerCard}>
            <p className={styles.overline}>Private Dispatch</p>
            <form className={styles.form} onSubmit={props.submitPrivateMessage}>
              <label className={styles.field}>
                <span className={styles.fieldLabel}>Agent</span>
                <select
                  className={styles.fieldControl}
                  value={props.privateAgentId}
                  onChange={(event) => props.setPrivateAgentId(event.target.value)}
                >
                  {props.agents.map((agent) => (
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
                  value={props.privateGoal}
                  placeholder={props.privatePlaceholders.goal}
                  onChange={(event) => props.setPrivateGoal(event.target.value)}
                />
              </label>
              <label className={styles.field}>
                <span className={styles.fieldLabel}>Message</span>
                <textarea
                  className={styles.textArea}
                  value={props.privateBody}
                  placeholder={props.privatePlaceholders.body}
                  onChange={(event) => props.setPrivateBody(event.target.value)}
                />
              </label>
              <button className={styles.primaryButton} type="submit">
                发送私聊消息
              </button>
            </form>
          </article>

          <article className={styles.composerCard}>
            <p className={styles.overline}>Space Dispatch</p>
            <form className={styles.form} onSubmit={props.submitSpaceMessage}>
              <label className={styles.field}>
                <span className={styles.fieldLabel}>Space</span>
                <select
                  className={styles.fieldControl}
                  value={props.selectedSpaceId}
                  onChange={(event) => props.setSelectedSpaceId(event.target.value)}
                >
                  {props.spaces.map((space) => (
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
                  value={props.spaceGoal}
                  placeholder={props.spacePlaceholders.goal}
                  onChange={(event) => props.setSpaceGoal(event.target.value)}
                />
              </label>
              <div className={styles.field}>
                <span className={styles.fieldLabel}>Addressed Agents</span>
                <div className={styles.chipRow}>
                  {props.agents.map((agent) => {
                    const active = props.selectedSpaceAgents.includes(agent.id);
                    return (
                      <button
                        key={agent.id}
                        className={`${styles.chip} ${active ? styles.chipActive : ""}`}
                        type="button"
                        onClick={() =>
                          props.setSelectedSpaceAgents(
                            active
                              ? props.selectedSpaceAgents.filter(
                                  (agentId) => agentId !== agent.id,
                                )
                              : [...props.selectedSpaceAgents, agent.id],
                          )
                        }
                      >
                        {agent.display_name}
                      </button>
                    );
                  })}
                </div>
              </div>
              <label className={styles.field}>
                <span className={styles.fieldLabel}>Message</span>
                <textarea
                  className={styles.textArea}
                  value={props.spaceBody}
                  placeholder={props.spacePlaceholders.body}
                  onChange={(event) => props.setSpaceBody(event.target.value)}
                />
              </label>
              <button className={styles.primaryButton} type="submit">
                发送群聊消息
              </button>
            </form>
          </article>

          <article className={styles.composerCard}>
            <p className={styles.overline}>Scheduler</p>
            <form className={styles.form} onSubmit={props.submitJob}>
              <label className={styles.field}>
                <span className={styles.fieldLabel}>Description</span>
                <input
                  className={styles.fieldControl}
                  value={props.jobDescription}
                  placeholder={props.jobPlaceholder}
                  onChange={(event) => props.setJobDescription(event.target.value)}
                />
              </label>
              <button className={styles.primaryButton} type="submit">
                注册后台任务
              </button>
            </form>
          </article>
        </div>

        <section className={styles.threadShell}>
          <div className={styles.chipRow}>
            {(props.selectedThread?.participants ?? []).map((participant) => (
              <span key={participant} className={styles.chip}>
                {participant}
              </span>
            ))}
          </div>

          <div className={styles.timeline}>
            {props.messages.length > 0 ? (
              props.messages.map((message) => (
                <article key={message.id} className={styles.timelineItem}>
                  <span className={styles.timelineRole}>{message.role}</span>
                  <strong>{message.sender}</strong>
                  <p className={styles.timelineBody}>{message.body}</p>
                </article>
              ))
            ) : (
              <div className={styles.emptyState}>
                选中线程后，这里会显示真实消息时间线。
              </div>
            )}
          </div>
        </section>
      </div>
    </article>
  );
}

export function ActivityRail(props: ActivityRailProps) {
  return (
    <aside className={styles.activityStack}>
      <PanelSection title="Runs" eyebrow="Execution">
        {props.runs.length > 0 ? (
          props.runs.map((run) => (
            <ListRow key={run.id} title={run.goal} meta={`${run.trigger} · ${run.status}`} />
          ))
        ) : (
          <EmptyState text="当前线程还没有 run。" />
        )}
      </PanelSection>

      <PanelSection title="Tasks" eyebrow="Assignments">
        {props.tasks.length > 0 ? (
          props.tasks.map((task) => (
            <ListRow
              key={task.id}
              title={task.title}
              meta={`${task.assigned_agent_id} · ${task.task_kind} · ${task.status}`}
            />
          ))
        ) : (
          <EmptyState text="当前线程还没有 task。" />
        )}
      </PanelSection>

      <PanelSection title="Memory" eyebrow="Context">
        {props.memories.length > 0 ? (
          props.memories.map((memory) => (
            <ListRow
              key={memory.id}
              title={memory.summary}
              meta={`${memory.source} · ${memory.owner.id}`}
            />
          ))
        ) : (
          <EmptyState text="记忆会在消息与 run 进入正式链路后出现在这里。" />
        )}
      </PanelSection>

      <PanelSection title="Artifacts" eyebrow="Outputs">
        {props.artifacts.length > 0 ? (
          props.artifacts.map((artifact) => (
            <ListRow
              key={artifact.id}
              title={artifact.relative_path}
              meta={`${artifact.kind} · ${artifact.owner.id}`}
            />
          ))
        ) : (
          <EmptyState text="当前线程还没有 artifact。" />
        )}
      </PanelSection>

      <PanelSection title="Jobs" eyebrow="Scheduler">
        {props.jobs.length > 0 ? (
          props.jobs.slice(0, 4).map((job) => (
            <ListRow
              key={job.id}
              title={job.description}
              meta={`${job.schedule_kind} · ${job.schedule_value}`}
            />
          ))
        ) : (
          <EmptyState text="注册后台任务后会出现在这里。" />
        )}
      </PanelSection>
    </aside>
  );
}
