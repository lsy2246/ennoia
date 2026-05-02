import React from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ExtensionUiModule } from "@ennoia/ui-sdk";

import WorkflowConversationCard from "./conversation/ConversationCard";
import WorkflowPage from "./page/Page";

const roots = new WeakMap<HTMLElement, Root>();

function renderIntoContainer(
  container: HTMLElement,
  node: React.ReactNode,
) {
  let root = roots.get(container);
  if (!root) {
    root = createRoot(container);
    roots.set(container, root);
  }
  root.render(<React.StrictMode>{node}</React.StrictMode>);
  return {
    unmount() {
      const current = roots.get(container);
      current?.unmount();
      roots.delete(container);
    },
  };
}

const extensionUi: ExtensionUiModule = {
  pages: {
    "workflow.page": (container, context) =>
      renderIntoContainer(
        container,
        <WorkflowPage helpers={context.helpers} />,
      ),
  },
  conversationCards: {
    "workflow.conversation.card": (container, context) =>
      renderIntoContainer(
        container,
        <WorkflowConversationCard
          conversationId={context.conversationId}
          helpers={context.helpers}
        />,
      ),
  },
};

export default extensionUi;
