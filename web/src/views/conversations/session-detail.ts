import type {
  ConversationDetail,
  ConversationMessageAppendResponse,
} from "@ennoia/api-client";

export function mergeConversationAppendResponse(
  detail: ConversationDetail,
  response: ConversationMessageAppendResponse,
): ConversationDetail {
  const nextMessages = detail.messages.some((message) => message.id === response.message.id)
    ? detail.messages
    : [...detail.messages, response.message].sort((left, right) =>
      left.created_at.localeCompare(right.created_at));

  return {
    ...detail,
    conversation: response.conversation,
    lanes: detail.lanes.some((lane) => lane.id === response.lane.id)
      ? detail.lanes.map((lane) => lane.id === response.lane.id ? response.lane : lane)
      : [...detail.lanes, response.lane],
    branches: detail.branches.some((branch) => branch.id === response.branch.id)
      ? detail.branches.map((branch) => branch.id === response.branch.id ? response.branch : branch)
      : [...detail.branches, response.branch],
    messages: nextMessages,
  };
}
