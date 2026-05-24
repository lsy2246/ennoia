import { describe, expect, test } from "bun:test";

import { buildPendingReplyStatusEntries, buildStatusEntries } from "./chat-entry-builder.ts";
import { resolveActiveConversationBranch } from "./session-branch.ts";
import { mergeConversationAppendResponse } from "./session-detail.ts";

describe("conversation session branch resolution", () => {
  test("keeps the conversation active branch id when rewrite branch details have not refreshed yet", () => {
    const active = resolveActiveConversationBranch({
      conversationActiveBranchId: "branch-new-rewrite",
      branches: [{
        id: "branch-old",
        conversation_id: "conv-1",
        name: "Old branch",
        kind: "main",
        status: "active",
        inherit_mode: "inclusive",
        created_at: "2026-05-24T10:00:00.000Z",
        updated_at: "2026-05-24T10:00:00.000Z",
      }],
    });

    expect(active.branchId).toBe("branch-new-rewrite");
    expect(active.branch?.id).toBe("branch-old");
  });

  test("merges the new rewrite branch and operator message before stream refresh arrives", () => {
    const detail = {
      conversation: {
        id: "conv-1",
        topology: "direct",
        owner: { kind: "global", id: "operator" },
        title: "Session",
        participants: ["operator", "agent-1"],
        active_branch_id: "branch-old",
        default_lane_id: "branch-old",
        created_at: "2026-05-24T10:00:00.000Z",
        updated_at: "2026-05-24T10:00:00.000Z",
      },
      lanes: [{
        id: "branch-old",
        conversation_id: "conv-1",
        name: "Old lane",
        lane_type: "branch",
        status: "active",
        goal: "",
        participants: ["agent-1"],
        created_at: "2026-05-24T10:00:00.000Z",
        updated_at: "2026-05-24T10:00:00.000Z",
      }],
      branches: [{
        id: "branch-old",
        conversation_id: "conv-1",
        name: "Old branch",
        kind: "main",
        status: "active",
        inherit_mode: "inclusive",
        created_at: "2026-05-24T10:00:00.000Z",
        updated_at: "2026-05-24T10:00:00.000Z",
      }],
      messages: [],
      records: [],
      operations: [],
      runs: [],
      tasks: [],
      outputs: [],
    };
    const next = mergeConversationAppendResponse(detail, {
      conversation: {
        ...detail.conversation,
        active_branch_id: "branch-new-rewrite",
        default_lane_id: "branch-new-rewrite",
        updated_at: "2026-05-24T10:01:00.000Z",
      },
      lane: {
        id: "branch-new-rewrite",
        conversation_id: "conv-1",
        name: "Rewrite lane",
        lane_type: "branch",
        status: "active",
        goal: "",
        participants: ["agent-1"],
        created_at: "2026-05-24T10:01:00.000Z",
        updated_at: "2026-05-24T10:01:00.000Z",
      },
      branch: {
        id: "branch-new-rewrite",
        conversation_id: "conv-1",
        name: "Rewrite branch",
        kind: "rewrite",
        status: "active",
        parent_branch_id: "branch-old",
        source_message_id: "msg-old",
        inherit_mode: "exclusive",
        created_at: "2026-05-24T10:01:00.000Z",
        updated_at: "2026-05-24T10:01:00.000Z",
      },
      message: {
        id: "msg-new",
        conversation_id: "conv-1",
        branch_id: "branch-new-rewrite",
        lane_id: "branch-new-rewrite",
        sender: "Operator",
        role: "operator",
        body: "rewrite body",
        mentions: ["agent-1"],
        rewrite_from_message_id: "msg-old",
        created_at: "2026-05-24T10:01:01.000Z",
      },
      tasks: [],
      artifacts: [],
    });

    expect(next.conversation.active_branch_id).toBe("branch-new-rewrite");
    expect(next.branches.some((branch) => branch.id === "branch-new-rewrite")).toBe(true);
    expect(next.messages.some((message) => message.id === "msg-new")).toBe(true);
  });

  test("builds a thinking status for a running operation on the active rewrite branch", () => {
    const active = resolveActiveConversationBranch({
      conversationActiveBranchId: "branch-new-rewrite",
      branches: [{
        id: "branch-old",
        conversation_id: "conv-1",
        name: "Old branch",
        kind: "main",
        status: "active",
        inherit_mode: "inclusive",
        created_at: "2026-05-24T10:00:00.000Z",
        updated_at: "2026-05-24T10:00:00.000Z",
      }],
    });
    const operations = [{
      id: "op-1",
      extension_id: "workflow",
      agent_id: "agent-1",
      conversation_id: "conv-1",
      branch_id: "branch-new-rewrite",
      lane_id: "branch-new-rewrite",
      run_id: "run-1",
      message_id: "msg-new",
      kind: "provider",
      name: "generate",
      status: "running",
      input: {},
      created_at: "2026-05-24T10:01:02.000Z",
      updated_at: "2026-05-24T10:01:03.000Z",
    }];
    const visibleOperations = operations.filter((operation) =>
      [operation.branch_id, operation.lane_id].includes(active.branchId));
    const entries = buildStatusEntries({
      operations: visibleOperations,
      resolveAgent: () => ({ id: "agent-1", display_name: "Agent One" }),
      texts: {
        typingLabel: "思考中",
        typingDetail: "Agent 已接到消息，正在组织回复与处理工具步骤。",
        operationGenerating: "正在生成回复。",
        operationCommand: "正在执行命令。",
        operationFileWrite: "正在写入文件。",
        operationFileRead: "正在读取文件。",
        operationNetwork: "正在请求网络资源。",
      },
    });

    expect(entries).toHaveLength(1);
    expect(entries[0].kind).toBe("status");
    expect(entries[0].label).toBe("思考中");
  });

  test("builds a thinking status from a pending rewrite reply before an operation snapshot arrives", () => {
    const entries = buildPendingReplyStatusEntries({
      pendingReplies: [{
        id: "msg-new:agent-1",
        agentId: "agent-1",
        branchId: "branch-new-rewrite",
        createdAt: "2026-05-24T10:01:01.000Z",
        sourceMessageId: "msg-new",
      }],
      activeBranchId: "branch-new-rewrite",
      operations: [],
      resolveAgent: () => ({ id: "agent-1", display_name: "Agent One" }),
      texts: {
        typingLabel: "思考中",
        typingDetail: "Agent 已接到消息，正在组织回复与处理工具步骤。",
        operationGenerating: "正在生成回复。",
        operationCommand: "正在执行命令。",
        operationFileWrite: "正在写入文件。",
        operationFileRead: "正在读取文件。",
        operationNetwork: "正在请求网络资源。",
      },
    });

    expect(entries).toHaveLength(1);
    expect(entries[0].kind).toBe("status");
    expect(entries[0].label).toBe("思考中");
    expect(entries[0].branchId).toBe("branch-new-rewrite");
    expect(entries[0].sourceMessageId).toBe("msg-new");
  });
});
